use super::{Lifetime, LifetimeCtx, LifetimeInfo, Side};
use crate::order::{Element, Relation, Transistor, TransistorConfig};
use std::fmt::Display;

impl LifetimeCtx {
  pub fn populate_bounds(&mut self, side: Side, source: impl Display) -> Result<(), String> {
    let bounds = Transistor::new(
      &self[side],
      TransistorConfig {
        enter: &|_, _, b| self.lifetimes[b].side == side,
        remap: &|_, r, b| (self.lifetimes[b].side != side).then_some(r),
        trans: &|_, r0, _, r1, _| r0 + r1,
      },
    )
    .finish_where(|a| self.lifetimes[a].side == side);

    for (a, info) in &mut self.lifetimes {
      if info.side != side {
        continue;
      }
      let Some(el) = bounds.els.get(a) else {
        continue;
      };
      info.min = Self::get_bound(info, el, side, Relation::gte_component, &source, "lower")?;
      info.max = Self::get_bound(info, el, side, Relation::lte_component, &source, "upper")?;
    }

    Ok(())
  }

  fn get_bound(
    info: &LifetimeInfo,
    el: &Element<Lifetime>,
    side: Side,
    component: impl Fn(Relation) -> Option<Relation>,
    name: impl Display,
    bound_type: impl Display,
  ) -> Result<Option<Lifetime>, String> {
    let mut bounds = el.rels.iter().filter_map(|(&b, &r)| Some((b, component(r)?)));
    Ok(if let Some((min, rel)) = bounds.next() {
      if bounds.next().is_some() {
        Err(format!(
          "in {name}, {side} lifetime `{lt}` has multiple {other_side} {bound_type} bounds
  rewrite the contract so there is only one
  (this is a temporary limitation of the checker)",
          lt = info.name,
          other_side = !side,
        ))?;
        todo!()
      }
      if !rel.allows_equal() {
        Err(format!(
          "in {name}, {side} lifetime `{lt}`'s {other_side} {bound_type} bound is related with `<`, not `<=`
  rewrite the contract so that it uses `<=`
  (this is a temporary limitation of the checker)",
          lt = info.name,
          other_side = !side,
        ))?;
      }
      Some(min)
    } else {
      None
    })
  }
}
