use std::{
  fmt::{Debug, Display},
  ops::{Add, BitXor, Index, IndexMut, Not},
};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Polarity {
  Pos,
  Neg,
}

impl Debug for Polarity {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Pos => write!(f, "+"),
      Self::Neg => write!(f, "-"),
    }
  }
}

pub use Polarity::*;

use crate::{index_vec::IndexVec, new_index, order::Order};

impl Not for Polarity {
  type Output = Polarity;

  fn not(self) -> Self::Output {
    match self {
      Pos => Neg,
      Neg => Pos,
    }
  }
}

impl Debug for Type {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}{}", self.polarity(), self.0 / 2)
  }
}

impl Type {
  pub fn polarity(&self) -> Polarity {
    match self.0 & 1 {
      0 => Pos,
      _ => Neg,
    }
  }
}

impl Not for Type {
  type Output = Type;

  fn not(self) -> Self::Output {
    Type(self.0 ^ 1)
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortLabel(pub Type, pub Lifetime);

impl Debug for PortLabel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}{:?}", self.0, self.1)
  }
}

#[derive(Debug, Clone)]
pub struct TypeInfo {
  pub name: String,
}

new_index!(pub Lifetime);
new_index!(pub Agent);
new_index!(pub Type);
new_index!(pub Var);

impl Debug for Var {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "v{}", self.0)
  }
}

impl Debug for Agent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "A{}", self.0)
  }
}

impl Debug for Lifetime {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "'{}", self.0)
  }
}

#[derive(Debug, Clone)]
pub struct AgentInfo {
  pub name: String,
  pub lt_ctx: LifetimeCtx,
  pub ports: Vec<PortLabel>,
}

#[derive(Debug, Clone)]
pub struct LifetimeInfo {
  pub name: String,
  pub side: Side,
  pub max: Option<Lifetime>,
  pub min: Option<Lifetime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
  External,
  Internal,
}

impl Display for Side {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Side::External => f.write_str("external"),
      Side::Internal => f.write_str("internal"),
    }
  }
}

impl Not for Side {
  type Output = Side;

  fn not(self) -> Self::Output {
    match self {
      Side::External => Side::Internal,
      Side::Internal => Side::External,
    }
  }
}

impl BitXor<bool> for Side {
  type Output = Side;

  fn bitxor(self, rhs: bool) -> Self::Output {
    if rhs {
      !self
    } else {
      self
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct VarCtx {
  pub vars: IndexVec<Var, VarInfo>,
}

#[derive(Debug, Clone)]
pub struct VarInfo {
  pub name: String,
  pub uses: Vec<PortLabel>,
}

#[derive(Debug, Clone)]
pub struct Node {
  pub agent: Agent,
  pub ports: Vec<Var>,
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

#[derive(Debug, Clone, Default)]
pub struct LifetimeCtx {
  pub lifetimes: IndexVec<Lifetime, LifetimeInfo>,
  pub ex_order: Order<Lifetime>,
  pub in_order: Order<Lifetime>,
}

impl Index<Side> for LifetimeCtx {
  type Output = Order<Lifetime>;

  fn index(&self, index: Side) -> &Self::Output {
    match index {
      Side::External => &self.ex_order,
      Side::Internal => &self.in_order,
    }
  }
}

impl IndexMut<Side> for LifetimeCtx {
  fn index_mut(&mut self, index: Side) -> &mut Self::Output {
    match index {
      Side::External => &mut self.ex_order,
      Side::Internal => &mut self.in_order,
    }
  }
}

impl LifetimeCtx {
  pub fn intro(&mut self, name: String, side: Side) -> Lifetime {
    self.lifetimes.push(LifetimeInfo { name, side, min: None, max: None })
  }

  pub fn show_lt<'a>(&'a self) -> impl Fn(Lifetime) -> &'a str {
    |lt| &self.lifetimes[lt].name
  }

  pub fn import(&mut self, from: &LifetimeCtx, invert: bool, prefix: impl Display) -> Lifetime {
    let base = self.lifetimes.len();
    for lt in from.lifetimes.values() {
      self.lifetimes.push(LifetimeInfo {
        name: format!("'{prefix}{}", &lt.name[1..]),
        side: lt.side ^ invert,
        min: lt.min.map(|lt| base + lt),
        max: lt.max.map(|lt| base + lt),
      });
    }
    let (known, needs) = if invert { (&from.in_order, &from.ex_order) } else { (&from.ex_order, &from.in_order) };
    self.ex_order.import(known, |lt| base + lt);
    self.in_order.import(needs, |lt| base + lt);
    base
  }
}

impl Add<Lifetime> for Lifetime {
  type Output = Lifetime;

  fn add(self, rhs: Lifetime) -> Self::Output {
    Lifetime(self.0 + rhs.0)
  }
}

#[derive(Debug, Clone, Default)]
pub struct Ctx {
  pub types: IndexVec<Type, TypeInfo>,
  pub agents: IndexVec<Agent, AgentInfo>,
  pub rules: Vec<RuleInfo>,
  pub nets: Vec<NetInfo>,
}

impl Ctx {
  pub fn show_type<'a>(&'a self) -> impl Fn(Type) -> &'a str {
    |ty| &self.types[ty].name
  }
}
