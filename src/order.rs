mod complete;
mod find_cycles;

use std::{
  cell::Cell,
  fmt::{Debug, Display},
};

use crate::{
  index_vec::{Idx, IndexVec},
  types::Polarity,
  util::{Captures, DisplayFn},
};
use nohash_hasher::IntMap;

#[derive(Clone)]
pub struct Order<I: Idx> {
  els: IndexVec<I, Element<I>>,
}

#[derive(Clone)]
struct Element<I: Idx> {
  rels: IntMap<I, bool>,
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
    for (a, b, eq) in self.iter() {
      let op = if eq { "<=" } else { "<" };
      f.entry(&format_args!("{a:?} {op} {b:?}"));
    }
    f.finish()
  }
}

impl<I: Idx> Extend<(I, I, bool)> for Order<I> {
  fn extend<T: IntoIterator<Item = (I, I, bool)>>(&mut self, iter: T) {
    for (a, b, eq) in iter {
      self.relate_lt(a, b, eq)
    }
  }
}

impl<I: Idx> FromIterator<(I, I, bool)> for Order<I> {
  fn from_iter<T: IntoIterator<Item = (I, I, bool)>>(iter: T) -> Self {
    let mut order = Order::default();
    order.extend(iter);
    order
  }
}

impl<I: Idx> Order<I> {
  pub fn relate_lt(&mut self, a: I, b: I, eq: bool) {
    // don't record `a <= a`
    if !(a == b && eq) {
      *self.els.get_or_extend(a).rels.entry(b).or_insert(true) &= eq;
    }
  }

  pub fn relate_polarity(&mut self, a: I, b: I, eq: bool, polarity: Polarity) {
    match polarity {
      Polarity::Pos => self.relate_lt(a, b, eq),
      Polarity::Neg => self.relate_lt(b, a, eq),
    }
  }

  pub fn import<J: Idx>(&mut self, from: &Order<J>, f: impl Fn(J) -> I) {
    for (a, b, eq) in from.iter() {
      self.relate_lt(f(a), f(b), eq);
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = (I, I, bool)> + '_ {
    self.els.iter().flat_map(|(a, el)| el.rels.iter().map(move |(&b, &eq)| (a, b, eq)))
  }

  fn clear_flags(&self) {
    for el in self.els.values() {
      el.flag.take();
    }
  }

  pub fn verify_empty<D: Display>(
    &self,
    base_message: impl Display,
    display_item: impl Fn(I) -> D,
  ) -> Result<(), String> {
    if self.els.values().any(|x| !x.rels.is_empty()) {
      use std::fmt::Write;
      let mut error = base_message.to_string();
      for (a, b, eq) in self.iter() {
        write!(&mut error, "\n  {} {} {}", display_item(a), show_relation(eq), display_item(b)).unwrap();
      }
      Err(error)
    } else {
      Ok(())
    }
  }

  pub fn verify_acyclic<D: Display>(
    &self,
    base_message: impl Display,
    display_item: impl Fn(I) -> D,
  ) -> Result<(), String> {
    let cycles = self.find_cycles();
    if !cycles.is_empty() {
      use std::fmt::Write;
      let mut error = base_message.to_string();
      for cycle in cycles {
        write!(&mut error, "\n  {}", self.show_cycle(cycle, &display_item)).unwrap();
      }
      Err(error)
    } else {
      Ok(())
    }
  }

  pub fn show_cycle<'a, D: Display>(
    &'a self,
    cycle: Vec<I>,
    display_item: impl Fn(I) -> D,
  ) -> impl Display + Captures<&'a ()>
  where
    I: 'a,
  {
    DisplayFn(move |f| {
      let mut last = None;
      for &b in &cycle {
        if let Some(a) = last {
          write!(f, " {} ", show_relation(self.els[a].rels[&b]))?;
        }
        write!(f, "{}", display_item(b))?;
        last = Some(b);
      }
      Ok(())
    })
  }
}

fn show_relation(eq: bool) -> &'static str {
  if eq {
    "<="
  } else {
    "<"
  }
}
