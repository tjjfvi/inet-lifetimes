use std::{borrow::Cow, fmt::Display};

use super::{Lifetime, LifetimeCtx, Side};
use crate::{display, error::Error, order::Order};

impl LifetimeCtx {
  pub fn check_contract_satisfiable(&mut self) -> Result<(), Error> {
    for side in [Side::External, Side::Internal] {
      self[side].check_coherent(self.show_lt()).report(format_args!("impossible {side} constraints:"))?;
    }

    for side in [Side::External, Side::Internal] {
      self.populate_bounds(side)?;
    }

    for side in [Side::External, Side::Internal] {
      let other_side = !side;

      self._check_satisfiable(
        Some(side),
        &self[other_side],
        &self[side],
        format_args!("satisfying {side} obligations would require incoherent constraints:"),
        format_args!("satisfying {side} obligations is impossible without more {other_side} guarantees:",),
      )?;
    }

    Ok(())
  }

  pub fn check_satisfiable(
    &self,
    side: Option<Side>,
    knows: &Order<Lifetime>,
    needs: &Order<Lifetime>,
  ) -> Result<(), Error> {
    #[allow(irrefutable_let_patterns)]
    let cycle_message = &display!("validity requires incoherent lifetime constraints:");
    let diff_message = &display!("validity requires constraints not guaranteed:");

    needs.check_coherent(self.show_lt()).report(cycle_message)?;
    self._check_satisfiable(side, knows, needs, cycle_message, diff_message)?;

    Ok(())
  }

  fn _check_satisfiable(
    &self,
    side: Option<Side>,
    knows: &Order<Lifetime>,
    needs: &Order<Lifetime>,
    cycle_message: impl Display,
    diff_message: impl Display,
  ) -> Result<(), Error> {
    needs.check_coherent(self.show_lt()).report(&cycle_message)?;

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
      new_knows.check_coherent(self.show_lt()).report(&cycle_message)?;
    }

    problems.verify_empty(self.show_lt()).report(diff_message)?;

    Ok(())
  }
}
