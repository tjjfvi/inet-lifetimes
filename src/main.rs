#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return)]

use std::{env, fs, process::exit};

use crate::{
  index_vec::IndexVec,
  order::Order,
  types::{Ctx, Lifetime, LifetimeCtx, PortLabel, Pos, Var},
  util::DisplayFn,
};

mod index_vec;
mod order;
mod parser;
mod types;
mod util;

fn _main() -> Result<(), String> {
  let path = env::args().skip(1).next().ok_or_else(|| format!("must supply path"))?;
  let src = String::from_utf8(fs::read(path).unwrap()).unwrap();
  let ctx: Ctx = src.parse()?;

  let mut ty_order = Order::default();

  for agent in ctx.agents.values() {
    let cycles = agent.lt_ctx.order.find_cycles();
    agent.lt_ctx.order.cycle_error(
      cycles,
      format_args!("impossible lifetime requirements in declaration of agent {}:", agent.name),
      agent.lt_ctx.show_lt(),
    )?;

    let mut required = Order::default();

    let pri = agent.ports[0];
    for aux in &agent.ports[1..] {
      if !aux.0 == pri.0 {
        required.relate_polarity(aux.1, pri.1, false, pri.0.polarity());
      } else {
        ty_order.relate_lt(!aux.0, pri.0, false);
      }
    }

    required.cycle_error(
      required.find_cycles(),
      format_args!("agent {} requires impossible lifetime constraints:", agent.name),
      agent.lt_ctx.show_lt(),
    )?;

    let diff = required.difference(&agent.lt_ctx.order);
    diff.diff_error(
      format_args!("agent {} requires constraints not present in declaration:", agent.name),
      agent.lt_ctx.show_lt(),
    )?;
  }

  let cycles = ty_order.find_cycles();
  ty_order.cycle_error(cycles, "found cycles in type order:", ctx.show_type())?;

  for rule in &ctx.rules {
    let a = &ctx.agents[rule.a.agent];
    let b = &ctx.agents[rule.b.agent];
    let rule_name = DisplayFn(|f| write!(f, "{}-{}", a.name, b.name));
    if rule.a.ports[0] != rule.b.ports[0] {
      Err(format!("nodes in {rule_name} are not connected by their principal ports"))?
    }

    let mut lt_ctx = LifetimeCtx::default();
    let a_base = lt_ctx.import(&a.lt_ctx, format_args!("{}.", a.name));
    let b_base = lt_ctx.import(&b.lt_ctx, format_args!("{}.", b.name));
    lt_ctx.order.relate_polarity(a_base + a.ports[0].1, b_base + b.ports[0].1, true, a.ports[0].0.polarity());

    let known = std::mem::take(&mut lt_ctx.order);

    let mut var_uses = IndexVec::<Var, Vec<PortLabel>>::default();

    for (lt_base, source_node) in [(a_base, &rule.a), (b_base, &rule.b)] {
      for (i, (&var, label)) in source_node.ports.iter().zip(&ctx.agents[source_node.agent].ports).enumerate() {
        var_uses.get_or_extend(var).push(PortLabel(if i != 0 { !label.0 } else { label.0 }, lt_base + label.1))
      }
    }

    let fresh_base = lt_ctx.lifetimes.len();
    for (i, node) in rule.result.iter().enumerate() {
      let lt_base = lt_ctx.import(&ctx.agents[node.agent].lt_ctx, format_args!("{i}."));
      for (&var, &label) in node.ports.iter().zip(&ctx.agents[node.agent].ports) {
        var_uses.get_or_extend(var).push(PortLabel(label.0, lt_base + label.1));
      }
    }

    let mut var_count_errors = String::new();
    for (var, uses) in var_uses {
      use std::fmt::Write;
      let name = &rule.var_ctx.vars[var].name;
      if uses.len() == 1 {
        write!(&mut var_count_errors, "\n  `{name}`: used only once").unwrap();
      } else if uses.len() > 2 {
        write!(&mut var_count_errors, "\n  `{name}`: used more than twice").unwrap();
      } else {
        let &[a, b] = &uses[..] else { unreachable!() };
        if a.0 != !b.0 {
          write!(
            &mut var_count_errors,
            "\n  `{name}`: mismatched types {} and {}",
            ctx.show_type()(a.0),
            ctx.show_type()(b.0)
          )
          .unwrap();
        } else {
          lt_ctx.order.relate_polarity(a.1, b.1, true, a.0.polarity());
        }
      }
    }

    if !var_count_errors.is_empty() {
      Err(format!("not all vars used twice in rule {rule_name}:{var_count_errors}"))?
    }

    let required = &lt_ctx.order;

    required.cycle_error(
      required.find_cycles(),
      format_args!("rule {rule_name} requires impossible lifetime constraints:"),
      lt_ctx.show_lt(),
    )?;

    let diff = required.truncate(fresh_base).difference(&known);
    diff
      .diff_error(format_args!("rule {rule_name} requires constraints not guaranteed by agents:"), lt_ctx.show_lt())?;
  }

  println!("ok");

  Ok(())
}

fn main() {
  if let Err(e) = _main() {
    println!("{}", e);
    exit(1);
  }
}
