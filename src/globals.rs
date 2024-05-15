use crate::{
  lifetimes::{Lifetime, LifetimeCtx},
  new_index,
  order::Order,
  scope::Scope,
};
use std::{fmt::Debug, ops::Not};

#[derive(Debug, Clone, Default)]
pub struct GlobalCtx {
  pub type_order: Order<Type>,
  pub types: Scope<Type, TypeInfo>,
  pub components: Scope<Component, ComponentInfo>,
}

new_index!(pub Type "type");

#[derive(Debug, Clone)]
pub struct TypeInfo {
  pub polarity: Polarity,
}

new_index!(pub Component "component");

#[derive(Debug, Clone)]
pub struct ComponentInfo {
  pub lt_ctx: LifetimeCtx,
  pub ports: Vec<PortLabel>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortLabel(pub Type, pub Lifetime);

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

impl Debug for Component {
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
    write!(f, "{}{}", if self.0 & 1 == 1 { "!" } else { "" }, self.0 / 2)
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
