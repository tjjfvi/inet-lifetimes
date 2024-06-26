mod find_cycles;
mod relation;
mod transistor;

pub use relation::*;
pub use transistor::*;

use std::{
  cell::Cell,
  fmt::{Debug, Display},
};

use crate::{
  err,
  error::{Error, ErrorGroup},
  globals::Polarity,
  index_vec::{Idx, IndexVec},
  util::DisplayFn,
};
use nohash_hasher::IntMap;

#[derive(Clone)]
pub struct Order<I: Idx> {
  pub els: IndexVec<I, Element<I>>,
}

#[derive(Clone)]
pub struct Element<I: Idx> {
  pub rels: IntMap<I, Relation>,
  flag: Cell<Flag>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Flag {
  #[default]
  None,
  Cycle(usize),
  Done,
}

impl<I: Idx> Default for Order<I> {
  fn default() -> Self {
    Self { els: Default::default() }
  }
}

impl<I: Idx> Default for Element<I> {
  fn default() -> Self {
    Self { rels: Default::default(), flag: Default::default() }
  }
}

impl<I: Idx + Debug> Debug for Order<I> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut f = f.debug_list();
    for (a, b, rel) in self.iter() {
      f.entry(&format_args!("{a:?} {rel:?} {b:?}"));
    }
    f.finish()
  }
}

impl<I: Idx> Extend<(I, I, Relation)> for Order<I> {
  fn extend<T: IntoIterator<Item = (I, I, Relation)>>(&mut self, iter: T) {
    for (a, b, rel) in iter {
      self.relate(a, b, rel)
    }
  }
}

impl<I: Idx> FromIterator<(I, I, Relation)> for Order<I> {
  fn from_iter<T: IntoIterator<Item = (I, I, Relation)>>(iter: T) -> Self {
    let mut order = Order::default();
    order.extend(iter);
    order
  }
}

impl<I: Idx> Order<I> {
  pub fn relate(&mut self, a: I, b: I, rel: Relation) {
    // don't record `a <= a`
    if !(a == b && rel.allows_equal()) {
      *self.els.get_or_extend(a).rels.entry(b).or_insert(rel) &= rel;
      *self.els.get_or_extend(b).rels.entry(a).or_insert(rel.rev()) &= rel.rev();
    }
  }

  pub fn relate_polarity(&mut self, a: I, b: I, rel: Relation, polarity: Polarity) {
    self.relate(
      a,
      b,
      match polarity {
        Polarity::Pos => rel,
        Polarity::Neg => rel.rev(),
      },
    )
  }

  pub fn import<J: Idx>(&mut self, from: &Order<J>, f: impl Fn(J) -> I) {
    for (a, b, rel) in from.iter() {
      if a < b {
        self.relate(f(a), f(b), rel);
      }
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = (I, I, Relation)> + '_ {
    self.els.iter().flat_map(|(a, el)| el.rels.iter().map(move |(&b, &rel)| (a, b, rel)))
  }

  pub fn iter_forward(&self) -> impl Iterator<Item = (I, I, Relation)> + '_ {
    self.iter().filter_map(|(a, b, rel)| Some((a, b, rel.lte_component()?)))
  }

  fn clear_flags(&self) {
    for el in self.els.values() {
      el.flag.take();
    }
  }

  pub fn verify_empty<D: Display>(&self, display_item: impl Fn(I) -> D) -> ErrorGroup {
    let mut errors = ErrorGroup::default();
    for (a, b, rel) in self.iter_forward() {
      errors.push(err!("{} {rel:?} {}", display_item(a), display_item(b)));
    }
    errors
  }

  pub fn check_coherent<D: Display>(&self, display_item: impl Fn(I) -> D) -> ErrorGroup {
    let mut errors = ErrorGroup::default();
    let cycles = self.find_cycles();
    for cycle in cycles {
      errors.push(self.show_cycle(cycle, &display_item));
    }
    errors
  }

  pub fn show_cycle<'a, D: Display>(&'a self, cycle: Vec<I>, display_item: impl Fn(I) -> D) -> Error
  where
    I: 'a,
  {
    Error::from(
      DisplayFn(move |f| {
        let mut last = None;
        for &b in &cycle {
          if let Some(a) = last {
            write!(f, " {:?} ", self.els[a].rels[&b].lte_component().unwrap())?;
          }
          write!(f, "{}", display_item(b))?;
          last = Some(b);
        }
        Ok(())
      })
      .to_string(),
    )
  }
}
