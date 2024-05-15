use crate::{
  globals::{Component, GlobalCtx, Polarity, PortLabel, Type},
  lifetimes::LifetimeCtx,
  vars::{Var, VarCtx},
};

mod check;

#[derive(Debug, Clone, Default)]
pub struct Program {
  pub globals: GlobalCtx,
  pub types: Vec<TypeDef>,
  pub agents: Vec<AgentDef>,
  pub rules: Vec<RuleDef>,
  pub nets: Vec<NetDef>,
}

#[derive(Debug, Clone)]
pub struct TypeDef {
  pub id: Type,
  pub polarity: Polarity,
}

#[derive(Debug, Clone)]
pub struct AgentDef {
  pub id: Component,
  pub lt_ctx: LifetimeCtx,
  pub ports: Vec<PortLabel>,
}

#[derive(Debug, Clone)]
pub struct RuleDef {
  pub var_ctx: VarCtx,
  pub a: Node,
  pub b: Node,
  pub result: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct NetDef {
  pub id: Component,
  pub var_ctx: VarCtx,
  pub lt_ctx: LifetimeCtx,
  pub free_ports: Vec<(Var, PortLabel)>,
  pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct Node {
  pub component: Component,
  pub ports: Vec<Var>,
}
