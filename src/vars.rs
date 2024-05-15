use crate::{
  err,
  error::{Error, ErrorGroup},
  globals::{GlobalCtx, PortLabel},
  lifetimes::LifetimeCtx,
  new_index,
  order::Relation,
  program::Node,
  scope::Scope,
  util::Captures,
};
use std::fmt::Debug;

new_index!(pub Var "variable");

#[derive(Debug, Clone, Default)]
pub struct VarCtx {
  pub vars: Scope<Var, VarInfo>,
}

#[derive(Debug, Clone, Default)]
pub struct VarInfo {
  pub uses: Vec<PortLabel>,
}

impl VarCtx {
  pub fn infer_uses(
    &mut self,
    errors: &mut ErrorGroup,
    globals: &GlobalCtx,
    lt_ctx: &mut LifetimeCtx,
    nodes: &Vec<Node>,
  ) {
    for (i, node) in nodes.iter().enumerate() {
      let lt_base = lt_ctx.import(&globals.components[node.component].lt_ctx, true, format_args!("{i}."));
      if let Some(pairs) = errors.push(self.check_node_arity(node, globals)) {
        for (var, label) in pairs {
          self.vars[var].uses.push(PortLabel(label.0, lt_base + label.1));
        }
      }
    }
  }

  pub fn check_types(&mut self, globals: &GlobalCtx, lt_ctx: &mut LifetimeCtx) -> ErrorGroup {
    let mut errors = ErrorGroup::default();
    for (_, name, VarInfo { uses }) in self.vars.iter() {
      if uses.len() == 1 {
        errors.push(err!("`{name}`: used only once"));
      } else if uses.len() > 2 {
        errors.push(err!("`{name}`: used more than twice"));
      } else {
        let &[a, b] = &uses[..] else { unreachable!() };
        if a.0 != !b.0 {
          errors.push(err!(
            "`{name}`: mismatched types `{}` and `{}`",
            globals.types.name(a.0),
            globals.types.name(b.0),
          ));
        } else {
          lt_ctx.in_order.relate_polarity(a.1, b.1, Relation::LE, globals.types[a.0].polarity);
        }
      }
    }
    errors
  }

  pub fn check_node_arity<'a>(
    &mut self,
    node: &'a Node,
    globals: &'a GlobalCtx,
  ) -> Result<impl Iterator<Item = (Var, PortLabel)> + Captures<&'a ()>, Error> {
    let signature = &globals.components[node.component].ports;
    if node.ports.len() == signature.len() {
      Ok(node.ports.iter().copied().zip(signature.iter().copied()))
    } else {
      for &var in &node.ports {
        self.vars.poison(var);
      }
      Err(err!(
        "`{}` expects {} ports but {} were supplied",
        globals.components.name(node.component),
        signature.len(),
        node.ports.len(),
      ))
    }
  }
}

impl Debug for Var {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "v{}", self.0)
  }
}
