#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return)]

use std::process::exit;

use crate::{
  order::Order,
  types::{Ctx, Pos},
};

mod index_vec;
mod order;
mod parser;
mod types;
mod util;

fn _main() -> Result<(), String> {
  let ctx: Ctx = include_str!("../examples/nat.inlt").parse()?;
  dbg!(&ctx);

  let mut ty_order = Order::default();

  for agent in &ctx.agents {
    let cycles = agent.lt_ctx.order.find_cycles();
    agent.lt_ctx.order.cycle_error(
      cycles,
      format_args!("impossible lifetime requirements in agent {}:", agent.name),
      agent.lt_ctx.show_lt(),
    )?;

    let mut required = Order::default();

    let pri = agent.ports[0];
    for aux in &agent.ports[1..] {
      if !aux.0 == pri.0 {
        if aux.0.polarity() == Pos {
          required.relate_lt(aux.1, pri.1, false);
        } else {
          required.relate_lt(pri.1, aux.1, false);
        }
      } else {
        ty_order.relate_lt(!aux.0, pri.0, false);
      }
    }
  }

  dbg!(&ty_order);
  let cycles = ty_order.find_cycles();
  ty_order.cycle_error(cycles, "found cycles in type order:", ctx.show_type())?;

  Ok(())
}

fn main() {
  if let Err(e) = _main() {
    println!("{}", e);
    exit(1);
  }
}
