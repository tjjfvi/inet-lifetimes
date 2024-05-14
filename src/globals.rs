use crate::{
  index_vec::IndexVec,
  lifetimes::{Lifetime, LifetimeCtx},
  new_index,
};
use std::{fmt::Debug, ops::Not};

#[derive(Debug, Clone, Default)]
pub struct GlobalCtx {
  pub types: IndexVec<Type, TypeInfo>,
  pub agents: IndexVec<Agent, AgentInfo>,
}

new_index!(pub Type);

#[derive(Debug, Clone)]
pub struct TypeInfo {
  pub name: String,
}

new_index!(pub Agent);

#[derive(Debug, Clone)]
pub struct AgentInfo {
  pub name: String,
  pub lt_ctx: LifetimeCtx,
  pub ports: Vec<PortLabel>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortLabel(pub Type, pub Lifetime);

impl GlobalCtx {
  pub fn show_type<'a>(&'a self) -> impl Fn(Type) -> &'a str {
    |ty| &self.types[ty].name
  }
}

impl Not for Type {
  type Output = Type;

  fn not(self) -> Self::Output {
    Type(self.0 ^ 1)
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Polarity {
  Pos,
  Neg,
}

impl Not for Polarity {
  type Output = Polarity;

  fn not(self) -> Self::Output {
    match self {
      Polarity::Pos => Polarity::Neg,
      Polarity::Neg => Polarity::Pos,
    }
  }
}
impl Type {
  pub fn polarity(&self) -> Polarity {
    match self.0 & 1 {
      0 => Polarity::Pos,
      _ => Polarity::Neg,
    }
  }
}

impl Debug for Agent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "A{}", self.0)
  }
}

impl Debug for PortLabel {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}{:?}", self.0, self.1)
  }
}

impl Debug for Type {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}{}", self.polarity(), self.0 / 2)
  }
}

impl Debug for Polarity {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::Pos => write!(f, "+"),
      Self::Neg => write!(f, "-"),
    }
  }
}
