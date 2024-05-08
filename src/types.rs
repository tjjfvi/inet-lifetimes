use std::{fmt::Debug, ops::Not};

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

impl Debug for Agent {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Agent").field(&self.0).finish()
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
}

#[derive(Debug, Clone, Default)]
pub struct LifetimeCtx {
  pub lifetimes: IndexVec<Lifetime, LifetimeInfo>,
  pub order: Order<Lifetime>,
}

impl LifetimeCtx {
  pub fn intro(&mut self, name: String) -> Lifetime {
    self.lifetimes.push(LifetimeInfo { name })
  }

  pub fn show_lt<'a>(&'a self) -> impl Fn(Lifetime) -> &'a str {
    |lt| &self.lifetimes[lt].name
  }
}

#[derive(Debug, Clone, Default)]
pub struct Ctx {
  pub types: IndexVec<Type, TypeInfo>,
  pub agents: Vec<AgentInfo>,
}

impl Ctx {
  pub fn show_type<'a>(&'a self) -> impl Fn(Type) -> &'a str {
    |ty| &self.types[ty].name
  }
}
