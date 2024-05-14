use std::{borrow::Cow, fmt::Display};

use super::{Lifetime, LifetimeCtx, Side};
use crate::order::Order;

impl LifetimeCtx {
  pub fn check_contract_satisfiable(&mut self, source: impl Display) -> Result<(), String> {
    for side in [Side::External, Side::Internal] {
      self[side].check_acyclic(format_args!("impossible {side} constraints in {source}:"), self.show_lt())?;
    }

    for side in [Side::External, Side::Internal] {
      self.populate_bounds(side, &source)?;
    }

    for side in [Side::External, Side::Internal] {
      let other_side = !side;

      self._check_satisfiable(
        Some(side),
        &self[other_side],
        &self[side],
        format!("in {source}, satisfying {side} obligations would require impossible constraints:"),
        format!("in {source}, satisfying {side} obligations is impossible without more {other_side} guarantees:",),
      )?;
    }

    Ok(())
  }

  pub fn check_satisfiable(
    &self,
    side: Option<Side>,
    knows: &Order<Lifetime>,
    needs: &Order<Lifetime>,
    source: impl Display,
  ) -> Result<(), String> {
    #[allow(irrefutable_let_patterns)]
    if let cycle_message = format_args!("validity of {source} requires impossible lifetime constraints:")
      && let diff_message = format_args!("validity of {source} would require constraints not guaranteed:")
    {
      needs.check_acyclic(&cycle_message, self.show_lt())?;
      self._check_satisfiable(side, knows, needs, cycle_message, diff_message)?;
    }

    Ok(())
  }

  fn _check_satisfiable(
    &self,
    side: Option<Side>,
    knows: &Order<Lifetime>,
    needs: &Order<Lifetime>,
    cycle_message: impl Display,
    diff_message: impl Display,
  ) -> Result<(), String> {
    needs.check_acyclic(&cycle_message, self.show_lt())?;

    let mut new_knows = Cow::Borrowed(knows);
    let mut problems = Order::default();
    for (a, b, rel_ab) in needs.omit(&|lt| Some(self.lifetimes[lt].side) == side).difference(&knows) {
      match (self.lifetimes[a].max, self.lifetimes[b].min) {
        (Some(x), Some(y)) => new_knows.to_mut().relate(x, y, rel_ab),
        (Some(x), None) => new_knows.to_mut().relate(x, b, rel_ab),
        (None, Some(y)) => new_knows.to_mut().relate(a, y, rel_ab),
        (None, None) => problems.relate(a, b, rel_ab),
      }
    }

    if matches!(new_knows, Cow::Owned(_)) {
      new_knows.check_acyclic(cycle_message, self.show_lt())?;
    }

    if let Err(mut err) = problems.verify_empty(diff_message, self.show_lt()) {
      use std::fmt::Write;

      write!(&mut err, "\n\nwe know:").unwrap();

      for lt in self.lifetimes.values() {
        if Some(lt.side) != side {
          write!(&mut err, " {}", lt.name).unwrap();
        }
      }

      for (a, b, rel) in knows.iter_forward() {
        write!(&mut err, "\n  {} {rel:?} {}", self.show_lt()(a), self.show_lt()(b)).unwrap();
      }

      write!(&mut err, "\n\nwe need:").unwrap();

      for lt in self.lifetimes.values() {
        if Some(lt.side) == side {
          write!(&mut err, " {}", lt.name).unwrap();
        }
      }

      for (a, b, rel) in needs.iter_forward() {
        write!(&mut err, "\n  {} {rel:?} {}", self.show_lt()(a), self.show_lt()(b)).unwrap();
      }

      Err(err)?
    }

    Ok(())
  }
}
