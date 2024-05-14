use crate::{
  globals::{GlobalCtx, PortLabel},
  index_vec::IndexVec,
  lifetimes::LifetimeCtx,
  new_index,
  order::Relation,
  program::Node,
};
use std::fmt::{Debug, Display};

new_index!(pub Var);

#[derive(Debug, Clone, Default)]
pub struct VarCtx {
  pub vars: IndexVec<Var, VarInfo>,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
  pub name: String,
  pub uses: Vec<PortLabel>,
}

impl VarCtx {
  pub fn infer_uses(&mut self, globals: &GlobalCtx, lt_ctx: &mut LifetimeCtx, nodes: &Vec<Node>) {
    for (i, node) in nodes.iter().enumerate() {
      let lt_base = lt_ctx.import(&globals.agents[node.agent].lt_ctx, true, format_args!("{i}."));
      for (&var, &label) in node.ports.iter().zip(&globals.agents[node.agent].ports) {
        self.vars[var].uses.push(PortLabel(label.0, lt_base + label.1));
      }
    }
  }

  pub fn check_types(
    &mut self,
    globals: &GlobalCtx,
    lt_ctx: &mut LifetimeCtx,
    source: impl Display,
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
          write!(
            &mut errors,
            "\n  `{name}`: mismatched types `{}` and `{}`",
            globals.types[a.0].name, globals.types[b.0].name
          )
          .unwrap();
        } else {
          lt_ctx.in_order.relate_polarity(a.1, b.1, Relation::LE, a.0.polarity());
        }
      }
    }
    Ok(if !errors.is_empty() {
      Err(format!("type errors in {source}:{errors}"))?
    })
  }
}

impl Debug for Var {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "v{}", self.0)
  }
}
