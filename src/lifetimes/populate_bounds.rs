use super::{Lifetime, LifetimeCtx, Side};
use crate::order::{Element, Relation, Transistor, TransistorConfig};

impl LifetimeCtx {
  pub fn populate_bounds(&mut self, side: Side) -> Result<(), String> {
    let bounds = Transistor::new(
      &self[side],
      TransistorConfig {
        enter: &|_, _, b| self.lifetimes[b].side == side,
        remap: &|_, r, b| (self.lifetimes[b].side != side).then_some(r),
        trans: &|_, r0, _, r1, _| r0 + r1,
      },
    )
    .finish_where(|a| self.lifetimes[a].side == side);

    for (a, name, info) in self.lifetimes.iter_mut() {
      if info.side != side {
        continue;
      }
      let Some(el) = bounds.els.get(a) else {
        continue;
      };
      info.min = Self::get_bound(name, el, side, Relation::gte_component, "lower")?;
      info.max = Self::get_bound(name, el, side, Relation::lte_component, "upper")?;
    }

    Ok(())
  }

  fn get_bound(
    lt: &str,
    el: &Element<Lifetime>,
    side: Side,
    component: impl Fn(Relation) -> Option<Relation>,
    bound_type: &str,
  ) -> Result<Option<Lifetime>, String> {
    let mut bounds = el.rels.iter().filter_map(|(&b, &r)| Some((b, component(r)?)));
    Ok(if let Some((min, rel)) = bounds.next() {
      if bounds.next().is_some() {
        Err(format!(
          "{side} lifetime `{lt}` has multiple {other_side} {bound_type} bounds
  rewrite the contract so there is only one
  (this is a temporary limitation of the checker)",
          other_side = !side,
        ))?;
        todo!()
      }
      if !rel.allows_equal() {
        Err(format!(
          "{side} lifetime `{lt}`'s {other_side} {bound_type} bound is related with `<`, not `<=`
  rewrite the contract so that it uses `<=`
  (this is a temporary limitation of the checker)",
          other_side = !side,
        ))?;
      }
      Some(min)
    } else {
      None
    })
  }
}
