use crate::{
  globals::GlobalCtx,
  globals::{Agent, PortLabel},
  lifetimes::LifetimeCtx,
  vars::Var,
  vars::VarCtx,
};

mod check;

#[derive(Debug, Clone, Default)]
pub struct Program {
  pub globals: GlobalCtx,
  pub rules: Vec<RuleInfo>,
  pub nets: Vec<NetInfo>,
}

#[derive(Debug, Clone)]
pub struct RuleInfo {
  pub var_ctx: VarCtx,
  pub a: Node,
  pub b: Node,
  pub result: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct NetInfo {
  pub name: String,
  pub var_ctx: VarCtx,
  pub lt_ctx: LifetimeCtx,
  pub free_ports: Vec<(Var, PortLabel)>,
  pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct Node {
  pub agent: Agent,
  pub ports: Vec<Var>,
}
