use crate::{
  display, err,
  error::{Error, ErrorGroup},
  globals::{ComponentInfo, GlobalCtx, PortLabel, TypeInfo},
  lifetimes::{LifetimeCtx, Side},
  order::{Order, Relation},
  program::{AgentDef, NetDef, Program, RuleDef, TypeDef},
};

impl Program {
  pub fn check(&mut self) -> ErrorGroup {
    let mut errors = ErrorGroup::default();

    for ty in &self.types {
      errors.push(ty.define(&mut self.globals));
    }

    for agent in &mut self.agents {
      errors.push(agent.define(&mut self.globals));
    }

    for net in &mut self.nets {
      errors.push(net.define(&mut self.globals));
    }

    for agent in self.agents.iter_mut() {
      if !self.globals.components.poisoned(agent.id) {
        errors.push(agent.check(&mut self.globals));
      }
    }

    errors
      .push(self.globals.type_order.check_coherent(|ty| self.globals.types.name(ty)).report("incoherent type order:"));

    for rule in self.rules.iter_mut() {
      errors.push(rule.check(&mut self.globals));
    }

    for net in self.nets.iter_mut() {
      if !self.globals.components.poisoned(net.id) {
        errors.push(net.check(&mut self.globals));
      }
    }

    errors
  }
}

impl TypeDef {
  fn define(&self, globals: &mut GlobalCtx) -> Result<(), Error> {
    globals.types.try_define(self.id, || TypeInfo { polarity: self.polarity })?;
    globals.types.try_define(!self.id, || TypeInfo { polarity: !self.polarity })?;
    Ok(())
  }
}

impl AgentDef {
  fn define(&mut self, globals: &mut GlobalCtx) -> Result<(), Error> {
    let mut errors = ErrorGroup::default();

    for port in &self.ports {
      errors.push(globals.types.get(port.0));
      errors.push(self.lt_ctx.lifetimes.get(port.1));
    }

    errors.push(self.lt_ctx.check_contract_satisfiable());

    errors.push(
      globals
        .components
        .try_define(self.id, || ComponentInfo { lt_ctx: self.lt_ctx.clone(), ports: self.ports.clone() }),
    );

    let name = globals.components.name(self.id);
    let ctx = display!("in agent `{name}`:");

    errors.report(ctx).map_err(|err| {
      globals.components.poison(self.id);
      err
    })
  }

  fn check(&mut self, globals: &mut GlobalCtx) -> Result<(), Error> {
    let mut required = Order::default();

    let pri = self.ports[0];
    for aux in &self.ports[1..] {
      if !aux.0 == pri.0 {
        required.relate_polarity(aux.1, pri.1, Relation::LT, globals.types[pri.0].polarity);
      } else {
        globals.type_order.relate(!aux.0, pri.0, Relation::LT);
      }
    }

    let mut full_order = self.lt_ctx.ex_order.clone();
    full_order.import(&self.lt_ctx.in_order, |x| x);

    let name = globals.components.name(self.id);
    let ctx = display!("in agent `{name}`:");

    self.lt_ctx.check_satisfiable(None, &full_order, &required).map_err(Error::context(ctx))
  }
}

impl RuleDef {
  fn check(&mut self, globals: &GlobalCtx) -> Result<(), Error> {
    let mut errors = ErrorGroup::default();

    let a_name = globals.components.name(self.a.component);
    let b_name = globals.components.name(self.a.component);
    let ctx = &display!("in rule `{a_name}-{b_name}`:");

    if self.a.ports[0] != self.b.ports[0] {
      errors.push(err!("matched nodes are not connected by their principal ports"));
    }

    let a = errors.push(globals.components.get(self.a.component));
    let b = errors.push(globals.components.get(self.b.component));

    for node in &self.result {
      errors.push(globals.components.get(node.component));
    }

    errors.report(ctx)?;

    let a = a.unwrap();
    let b = b.unwrap();

    let mut lt_ctx = LifetimeCtx::default();
    let a_base = lt_ctx.import(&a.lt_ctx, false, format_args!("{}.", a_name));
    let b_base = lt_ctx.import(&b.lt_ctx, false, format_args!("{}.", b_name));
    lt_ctx.ex_order.relate_polarity(
      a_base + a.ports[0].1,
      b_base + b.ports[0].1,
      Relation::LE,
      globals.types[a.ports[0].0].polarity,
    );

    for (lt_base, source_node) in [(a_base, &self.a), (b_base, &self.b)] {
      if let Some(pairs) = errors.push(self.var_ctx.check_node_arity(source_node, globals)) {
        for (i, (var, label)) in pairs.enumerate() {
          self.var_ctx.vars[var].uses.push(PortLabel(if i != 0 { !label.0 } else { label.0 }, lt_base + label.1))
        }
      }
    }

    self.var_ctx.infer_uses(&mut errors, &globals, &mut lt_ctx, &self.result);
    errors.push(self.var_ctx.check_types(&globals, &mut lt_ctx));

    errors.push(lt_ctx.check_satisfiable(Some(Side::Internal), &lt_ctx.ex_order, &lt_ctx.in_order));

    errors.report(ctx)
  }
}

impl NetDef {
  fn define(&mut self, globals: &mut GlobalCtx) -> Result<(), Error> {
    let mut errors = ErrorGroup::default();

    for (_, port) in &self.free_ports {
      errors.push(globals.types.get(port.0));
      errors.push(self.lt_ctx.lifetimes.get(port.1));
    }

    errors.push(self.lt_ctx.check_contract_satisfiable());

    errors.push(globals.components.try_define(self.id, || ComponentInfo {
      lt_ctx: self.lt_ctx.clone(),
      ports: self.free_ports.iter().map(|x| x.1).collect(),
    }));

    let name = globals.components.name(self.id);
    let ctx = display!("in net `{name}`:");

    errors.report(ctx).map_err(|err| {
      globals.components.poison(self.id);
      err
    })
  }

  fn check(&mut self, globals: &GlobalCtx) -> Result<(), Error> {
    let mut errors = ErrorGroup::default();

    let name = globals.components.name(self.id);
    let ctx = &display!("in net `{name}`:");

    for node in &self.nodes {
      errors.push(globals.components.get(node.component));
    }

    errors.report(ctx)?;

    for &(var, label) in &self.free_ports {
      self.var_ctx.vars[var].uses.push(PortLabel(!label.0, label.1))
    }

    self.var_ctx.infer_uses(&mut errors, &globals, &mut self.lt_ctx, &self.nodes);
    errors.push(self.var_ctx.check_types(&globals, &mut self.lt_ctx));

    errors.push(self.lt_ctx.check_satisfiable(Some(Side::Internal), &self.lt_ctx.ex_order, &self.lt_ctx.in_order));

    errors.report(ctx)
  }
}
