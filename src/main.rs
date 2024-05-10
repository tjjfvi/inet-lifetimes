#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return, const_option)]

use std::{env, fmt::Display, fs, process::ExitCode};

use order::Relation;

use crate::{
  index_vec::IndexVec,
  order::Order,
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
    agent.lt_ctx.full_order.verify_acyclic(
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

    agent.lt_ctx.verify_implication(
      &agent.lt_ctx.full_order,
      &required,
      format_args!("validity of agent `{}` would require impossible lifetime constraints:", agent.name),
      format_args!("validity of agent `{}` requires constraints not present in declaration:", agent.name),
    )?;

    agent.lt_ctx.split_full_order();
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
    lt_ctx.known_order.relate_polarity(
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
      &lt_ctx.known_order,
      &lt_ctx.needs_order,
      format_args!("validity of rule `{rule_name}` would require impossible lifetime constraints:"),
      format_args!("validity of rule `{rule_name}` would require constraints not guaranteed by agents:"),
    )?;
  }

  for net in ctx.nets.iter_mut() {
    let name = &net.name;

    net.lt_ctx.full_order.verify_acyclic(
      format_args!("impossible lifetime requirements in declaration of net `{name}`:"),
      net.lt_ctx.show_lt(),
    )?;

    net.lt_ctx.split_full_order();

    for &(var, label) in &net.free_ports {
      net.var_ctx.vars[var].uses.push(PortLabel(!label.0, label.1))
    }

    net.var_ctx.infer_uses(&net.nodes, &ctx.agents, &mut net.lt_ctx);
    net.var_ctx.verify_types(&ctx.types, &mut net.lt_ctx, format_args!("type errors in net `{name}`:"))?;

    net.lt_ctx.verify_implication(
      &net.lt_ctx.known_order,
      &net.lt_ctx.needs_order,
      format_args!("validity of net `{name}` would require impossible lifetime constraints:"),
      format_args!("validity of net `{name}` would require constraints not guaranteed:"),
    )?;
  }

  Ok(())
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
          lt_ctx.needs_order.relate_polarity(a.1, b.1, Relation::LE, a.0.polarity());
        }
      }
    }
    Ok(if !errors.is_empty() {
      Err(format!("{base_message}{errors}"))?
    })
  }
}

impl LifetimeCtx {
  fn split_full_order(&mut self) {
    self.known_order = self.full_order.omit(&|lt| !self.lifetimes[lt].fixed);
    self.needs_order = self.full_order.difference(&self.known_order);
  }

  fn verify_implication(
    &self,
    knows: &Order<Lifetime>,
    needs: &Order<Lifetime>,
    cycle_message: impl Display,
    diff_message: impl Display,
  ) -> Result<(), String> {
    needs.verify_acyclic(cycle_message, self.show_lt())?;

    let diff = needs.omit(&|lt| !self.lifetimes[lt].fixed).difference(&knows);
    if let Err(mut err) = diff.verify_empty(diff_message, self.show_lt()) {
      use std::fmt::Write;

      write!(&mut err, "\n\nwe know:").unwrap();

      for lt in self.lifetimes.values() {
        if lt.fixed {
          write!(&mut err, " {}", lt.name).unwrap();
        }
      }

      for (a, b, rel) in knows.iter_forward() {
        write!(&mut err, "\n  {} {rel:?} {}", self.show_lt()(a), self.show_lt()(b)).unwrap();
      }

      write!(&mut err, "\n\nwe need:").unwrap();

      for lt in self.lifetimes.values() {
        if !lt.fixed {
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
