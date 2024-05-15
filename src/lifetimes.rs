mod check_satisfiable;
mod populate_bounds;

use crate::{new_index, order::Order, scope::Scope};
use std::{
  fmt::{Debug, Display},
  ops::{Add, BitXor, Index, IndexMut, Not},
};

#[derive(Debug, Clone, Default)]
pub struct LifetimeCtx {
  pub lifetimes: Scope<Lifetime, LifetimeInfo>,
  pub ex_order: Order<Lifetime>,
  pub in_order: Order<Lifetime>,
}

new_index!(pub Lifetime "lifetime");

#[derive(Debug, Clone)]
pub struct LifetimeInfo {
  pub side: Side,
  pub max: Option<Lifetime>,
  pub min: Option<Lifetime>,
}

impl LifetimeCtx {
  pub fn show_lt<'a>(&'a self) -> impl Fn(Lifetime) -> &'a str {
    |lt| self.lifetimes.name(lt)
  }

  pub fn import(&mut self, from: &LifetimeCtx, invert: bool, prefix: impl Display) -> Lifetime {
    let base = self.lifetimes.len();
    for (_, name, info) in from.lifetimes.iter() {
      self.lifetimes.push(
        format!("'{prefix}{}", &name[1..]),
        Some(LifetimeInfo {
          side: info.side ^ invert,
          min: info.min.map(|lt| base + lt),
          max: info.max.map(|lt| base + lt),
        }),
      );
    }
    let (known, needs) = if invert { (&from.in_order, &from.ex_order) } else { (&from.ex_order, &from.in_order) };
    self.ex_order.import(known, |lt| base + lt);
    self.in_order.import(needs, |lt| base + lt);
    base
  }
}

impl Debug for Lifetime {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "'{}", self.0)
  }
}

impl Add<Lifetime> for Lifetime {
  type Output = Lifetime;

  fn add(self, rhs: Lifetime) -> Self::Output {
    Lifetime(self.0 + rhs.0)
  }
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
