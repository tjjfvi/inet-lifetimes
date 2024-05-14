use crate::{
  globals::PortLabel,
  lifetimes::{LifetimeCtx, Side},
  order::{Order, Relation},
  util::DisplayFn,
};

use super::Program;

impl Program {
  pub fn check(&mut self) -> Result<(), String> {
    let mut ty_order = Order::default();

    for agent in self.globals.agents.values_mut() {
      agent.lt_ctx.check_contract_satisfiable(format_args!("agent `{}`", agent.name))?;

      let mut required = Order::default();

      let pri = agent.ports[0];
      for aux in &agent.ports[1..] {
        if !aux.0 == pri.0 {
          required.relate_polarity(aux.1, pri.1, Relation::LT, pri.0.polarity());
        } else {
          ty_order.relate(!aux.0, pri.0, Relation::LT);
        }
      }

      let mut full_order = agent.lt_ctx.ex_order.clone();
      full_order.import(&agent.lt_ctx.in_order, |x| x);

      agent.lt_ctx.check_satisfiable(None, &full_order, &required, format_args!("agent `{}`", agent.name))?;
    }

    ty_order.check_acyclic("found cycles in type order:", self.globals.show_type())?;

    for rule in self.rules.iter_mut() {
      let a = &self.globals.agents[rule.a.agent];
      let b = &self.globals.agents[rule.b.agent];
      let rule_name = DisplayFn(|f| write!(f, "{}-{}", a.name, b.name));
      if rule.a.ports[0] != rule.b.ports[0] {
        Err(format!("nodes in `{rule_name}` are not connected by their principal ports"))?
      }

      let mut lt_ctx = LifetimeCtx::default();
      let a_base = lt_ctx.import(&a.lt_ctx, false, format_args!("{}.", a.name));
      let b_base = lt_ctx.import(&b.lt_ctx, false, format_args!("{}.", b.name));
      lt_ctx.ex_order.relate_polarity(
        a_base + a.ports[0].1,
        b_base + b.ports[0].1,
        Relation::LE,
        a.ports[0].0.polarity(),
      );

      for (lt_base, source_node) in [(a_base, &rule.a), (b_base, &rule.b)] {
        for (i, (&var, label)) in
          source_node.ports.iter().zip(&self.globals.agents[source_node.agent].ports).enumerate()
        {
          rule.var_ctx.vars[var].uses.push(PortLabel(if i != 0 { !label.0 } else { label.0 }, lt_base + label.1))
        }
      }

      rule.var_ctx.infer_uses(&self.globals, &mut lt_ctx, &rule.result);
      rule.var_ctx.check_types(&self.globals, &mut lt_ctx, format_args!("rule `{rule_name}`"))?;

      lt_ctx.check_satisfiable(
        Some(Side::Internal),
        &lt_ctx.ex_order,
        &lt_ctx.in_order,
        format_args!("rule `{rule_name}`"),
      )?;
    }

    for net in self.nets.iter_mut() {
      let name = &net.name;

      net.lt_ctx.check_contract_satisfiable(format_args!("net `{name}`"))?;

      for &(var, label) in &net.free_ports {
        net.var_ctx.vars[var].uses.push(PortLabel(!label.0, label.1))
      }

      net.var_ctx.infer_uses(&self.globals, &mut net.lt_ctx, &net.nodes);
      net.var_ctx.check_types(&self.globals, &mut net.lt_ctx, format_args!("net `{name}`"))?;

      net.lt_ctx.check_satisfiable(
        Some(Side::Internal),
        &net.lt_ctx.ex_order,
        &net.lt_ctx.in_order,
        format_args!("net `{name}`"),
      )?;
    }

    Ok(())
  }
}
