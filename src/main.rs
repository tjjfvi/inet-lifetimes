#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return, const_option)]

use std::{borrow::Cow, env, fmt::Display, fs, process::ExitCode};

use order::{Element, Relation};
use types::{LifetimeInfo, Side};

use crate::{
  index_vec::IndexVec,
  order::{Order, Transistor, TransistorConfig},
  types::{Agent, AgentInfo, Ctx, Lifetime, LifetimeCtx, Node, PortLabel, Type, TypeInfo, VarCtx, VarInfo},
  util::DisplayFn,
};

mod index_vec;
mod order;
mod parser;
mod types;
mod util;

fn _main(path: &str) -> Result<(), String> {
  let src = String::from_utf8(fs::read(path).unwrap()).unwrap();
  let mut ctx: Ctx = src.parse()?;

  let mut ty_order = Order::default();

  for agent in ctx.agents.values_mut() {
    agent.lt_ctx.ex_order.verify_acyclic(
      format_args!("impossible lifetime requirements in declaration of agent `{}`:", agent.name),
      agent.lt_ctx.show_lt(),
    )?;
    agent.lt_ctx.in_order.verify_acyclic(
      format_args!("impossible lifetime requirements in declaration of agent `{}`:", agent.name),
      agent.lt_ctx.show_lt(),
    )?;

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

    agent.lt_ctx.verify_implication(
      None,
      &full_order,
      &required,
      format_args!("validity of agent `{}` would require impossible lifetime constraints:", agent.name),
      format_args!("validity of agent `{}` requires constraints not present in declaration:", agent.name),
    )?;

    agent.lt_ctx.populate_bounds(Side::Internal, format_args!("agent `{}`", agent.name))?;
    agent.lt_ctx.populate_bounds(Side::External, format_args!("agent `{}`", agent.name))?;
  }

  ty_order.verify_acyclic("found cycles in type order:", ctx.show_type())?;

  for rule in ctx.rules.iter_mut() {
    let a = &ctx.agents[rule.a.agent];
    let b = &ctx.agents[rule.b.agent];
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
      for (i, (&var, label)) in source_node.ports.iter().zip(&ctx.agents[source_node.agent].ports).enumerate() {
        rule.var_ctx.vars[var].uses.push(PortLabel(if i != 0 { !label.0 } else { label.0 }, lt_base + label.1))
      }
    }

    rule.var_ctx.infer_uses(&rule.result, &ctx.agents, &mut lt_ctx);
    rule.var_ctx.verify_types(&ctx.types, &mut lt_ctx, format_args!("type errors in rule `{rule_name}`:"))?;

    lt_ctx.verify_implication(
      Some(Side::Internal),
      &lt_ctx.ex_order,
      &lt_ctx.in_order,
      format_args!("validity of rule `{rule_name}` would require impossible lifetime constraints:"),
      format_args!("validity of rule `{rule_name}` would require constraints not guaranteed by agents:"),
    )?;
  }

  for net in ctx.nets.iter_mut() {
    let name = &net.name;

    net.lt_ctx.ex_order.verify_acyclic(
      format_args!("impossible lifetime requirements in declaration of net `{name}`:"),
      net.lt_ctx.show_lt(),
    )?;
    net.lt_ctx.in_order.verify_acyclic(
      format_args!("impossible lifetime requirements in declaration of net `{name}`:"),
      net.lt_ctx.show_lt(),
    )?;

    for &(var, label) in &net.free_ports {
      net.var_ctx.vars[var].uses.push(PortLabel(!label.0, label.1))
    }

    net.var_ctx.infer_uses(&net.nodes, &ctx.agents, &mut net.lt_ctx);
    net.var_ctx.verify_types(&ctx.types, &mut net.lt_ctx, format_args!("type errors in net `{name}`:"))?;

    net.lt_ctx.verify_implication(
      Some(Side::Internal),
      &net.lt_ctx.ex_order,
      &net.lt_ctx.in_order,
      format_args!("validity of net `{name}` would require impossible lifetime constraints:"),
      format_args!("validity of net `{name}` would require constraints not guaranteed:"),
    )?;
  }

  Ok(())
}

impl LifetimeCtx {
  fn populate_bounds(&mut self, side: Side, name: impl Display) -> Result<(), String> {
    let bounds = Transistor::new(
      &self[side],
      TransistorConfig {
        enter: &|_, _, b| self.lifetimes[b].side == side,
        remap: &|_, r, b| (self.lifetimes[b].side != side).then_some(r),
        trans: &|_, r0, _, r1, _| r0 + r1,
      },
    )
    .finish_where(|a| self.lifetimes[a].side == side);

    for (a, info) in &mut self.lifetimes {
      if info.side != side {
        continue;
      }
      let Some(el) = bounds.els.get(a) else {
        continue;
      };
      info.min = Self::get_bound(info, el, side, Relation::gte_component, &name, "lower")?;
      info.max = Self::get_bound(info, el, side, Relation::lte_component, &name, "upper")?;
    }

    Ok(())
  }

  fn get_bound(
    info: &LifetimeInfo,
    el: &Element<Lifetime>,
    side: Side,
    component: impl Fn(Relation) -> Option<Relation>,
    name: impl Display,
    bound_type: impl Display,
  ) -> Result<Option<Lifetime>, String> {
    let mut bounds = el.rels.iter().filter_map(|(&b, &r)| Some((b, component(r)?)));
    Ok(if let Some((min, rel)) = bounds.next() {
      if bounds.next().is_some() {
        Err(format!(
          "in {name}, {side} lifetime `{lt}` has multiple {other_side} {bound_type} bounds
  rewrite the contract so there is only one
  (this is a temporary limitation of the checker)",
          lt = info.name,
          other_side = !side,
        ))?;
        todo!()
      }
      if !rel.allows_equal() {
        Err(format!(
          "in {name}, {side} lifetime `{lt}`'s {other_side} {bound_type} bound is related with `<`, not `<=`
  rewrite the contract so that it uses `<=`
  (this is a temporary limitation of the checker)",
          lt = info.name,
          other_side = !side,
        ))?;
      }
      Some(min)
    } else {
      None
    })
  }
}

impl VarCtx {
  fn infer_uses(&mut self, nodes: &Vec<Node>, agents: &IndexVec<Agent, AgentInfo>, lt_ctx: &mut LifetimeCtx) {
    for (i, node) in nodes.iter().enumerate() {
      let lt_base = lt_ctx.import(&agents[node.agent].lt_ctx, true, format_args!("{i}."));
      for (&var, &label) in node.ports.iter().zip(&agents[node.agent].ports) {
        self.vars[var].uses.push(PortLabel(label.0, lt_base + label.1));
      }
    }
  }

  fn verify_types(
    &mut self,
    types: &IndexVec<Type, TypeInfo>,
    lt_ctx: &mut LifetimeCtx,
    base_message: impl Display,
  ) -> Result<(), String> {
    let mut errors = String::new();
    for VarInfo { name, uses } in self.vars.values() {
      use std::fmt::Write;
      if uses.len() == 1 {
        write!(&mut errors, "\n  `{name}`: used only once").unwrap();
      } else if uses.len() > 2 {
        write!(&mut errors, "\n  `{name}`: used more than twice").unwrap();
      } else {
        let &[a, b] = &uses[..] else { unreachable!() };
        if a.0 != !b.0 {
          write!(&mut errors, "\n  `{name}`: mismatched types `{}` and `{}`", types[a.0].name, types[b.0].name,)
            .unwrap();
        } else {
          lt_ctx.in_order.relate_polarity(a.1, b.1, Relation::LE, a.0.polarity());
        }
      }
    }
    Ok(if !errors.is_empty() {
      Err(format!("{base_message}{errors}"))?
    })
  }
}

impl LifetimeCtx {
  fn verify_implication(
    &self,
    side: Option<Side>,
    knows: &Order<Lifetime>,
    needs: &Order<Lifetime>,
    cycle_message: impl Display,
    diff_message: impl Display,
  ) -> Result<(), String> {
    needs.verify_acyclic(&cycle_message, self.show_lt())?;

    let mut new_knows = Cow::Borrowed(knows);
    let mut problems = Order::default();
    for (a, b, rel_ab) in needs.omit(&|lt| Some(self.lifetimes[lt].side) == side).difference(&knows) {
      match (self.lifetimes[a].max, self.lifetimes[b].min) {
        (Some(x), Some(y)) => new_knows.to_mut().relate(x, y, rel_ab),
        (Some(x), None) => new_knows.to_mut().relate(x, b, rel_ab),
        (None, Some(y)) => new_knows.to_mut().relate(a, y, rel_ab),
        (None, None) => problems.relate(a, b, rel_ab),
      }
    }

    new_knows.verify_acyclic(cycle_message, self.show_lt())?;

    if let Err(mut err) = problems.verify_empty(diff_message, self.show_lt()) {
      use std::fmt::Write;

      write!(&mut err, "\n\nwe know:").unwrap();

      for lt in self.lifetimes.values() {
        if Some(lt.side) != side {
          write!(&mut err, " {}", lt.name).unwrap();
        }
      }

      for (a, b, rel) in knows.iter_forward() {
        write!(&mut err, "\n  {} {rel:?} {}", self.show_lt()(a), self.show_lt()(b)).unwrap();
      }

      write!(&mut err, "\n\nwe need:").unwrap();

      for lt in self.lifetimes.values() {
        if Some(lt.side) == side {
          write!(&mut err, " {}", lt.name).unwrap();
        }
      }

      for (a, b, rel) in needs.iter_forward() {
        write!(&mut err, "\n  {} {rel:?} {}", self.show_lt()(a), self.show_lt()(b)).unwrap();
      }

      Err(err)?
    }

    Ok(())
  }
}

fn main() -> ExitCode {
  let mut any = false;
  let mut code = ExitCode::SUCCESS;
  for path in env::args().skip(1) {
    any = true;
    if let Err(e) = _main(&path) {
      println!("{path}:\n\n{}\n\n", e);
      code = ExitCode::FAILURE;
    } else {
      println!("{path}: ok")
    }
  }
  if !any {
    println!("supply a path");
    code = ExitCode::FAILURE;
  }
  code
}
