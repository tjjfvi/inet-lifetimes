use nohash_hasher::IntMap;

use crate::index_vec::Idx;

use super::{Flag, Order, Relation};

impl<I: Idx> Order<I> {
  #[allow(unused)]
  pub fn complete(&self) -> Order<I> {
    self.clear_flags();
    let mut completionist = Completionist { source: self, output: Order::default(), omit: None };
    for (a, _) in &self.els {
      completionist.visit(a, Some(0));
    }
    completionist.output
  }

  pub fn difference(&self, other: &Order<I>) -> Order<I> {
    other.clear_flags();
    let mut completionist = Completionist { source: other, output: Order::default(), omit: None };

    let mut diff = Order::default();
    for (a, b, rel) in self.iter_forward() {
      if !other.has(a, b, rel) {
        completionist.visit(a, Some(0));
        if !completionist.output.has(a, b, rel) {
          diff.relate(a, b, rel)
        }
      }
    }

    diff
  }

  pub fn omit(&self, omit: &dyn Fn(I) -> bool) -> Order<I> {
    self.clear_flags();
    let mut completionist = Completionist { source: self, output: Order::default(), omit: Some(omit) };
    for (a, _) in self.els.iter() {
      if !omit(a) {
        completionist.visit(a, Some(0));
      }
    }
    for (a, _) in self.els.iter() {
      if omit(a) {
        if let Some(el) = completionist.output.els.get_mut(a) {
          *el = Default::default();
        }
      }
    }
    completionist.output
  }

  fn has(&self, a: I, b: I, rel: Relation) -> bool {
    self.els.get(a).and_then(|x| x.rels.get(&b)).is_some_and(|&has_rel| (has_rel & rel) == has_rel)
  }
}

struct Completionist<'a, I: Idx> {
  source: &'a Order<I>,
  output: Order<I>,
  omit: Option<&'a dyn Fn(I) -> bool>,
}

impl<'a, I: Idx> Completionist<'a, I> {
  fn visit(&mut self, a: I, depth: Option<usize>) -> usize {
    let Some(el) = self.source.els.get(a) else { return usize::MAX };
    match el.flag.get() {
      Flag::None => el.flag.set(Flag::Cycle(depth.unwrap())),
      Flag::Done => return usize::MAX,
      Flag::Cycle(d) if depth.is_some() => return d,
      Flag::Cycle(_) => el.flag.set(Flag::Done),
    }

    let mut head_depth = usize::MAX;

    for (b, _) in el.forward_rels() {
      if !self.omit.is_some_and(|omit| !omit(b)) {
        head_depth = head_depth.min(self.visit(b, depth.map(|x| x + 1)));
      }
    }

    if depth.is_some_and(|x| x > head_depth) {
      el.flag.set(Flag::Cycle(head_depth));
    } else {
      let mut rels = IntMap::default();
      for (b, rel_0) in el.forward_rels() {
        if !self.omit.is_some_and(|omit| omit(b)) {
          rels.insert(b, rel_0);
        }
        if !self.omit.is_some_and(|omit| !omit(b)) {
          if let Some(other) = self.output.els.get(b) {
            for (c, rel_1) in other.forward_rels() {
              rels.insert(c, rel_0 & rel_1);
            }
          }
        }
      }
      self.output.els.get_or_extend(a).rels = rels;

      el.flag.set(Flag::Done);
      if depth == Some(head_depth) {
        for (b, _) in el.forward_rels() {
          if !self.omit.is_some_and(|omit| !omit(b)) {
            self.visit(b, depth.map(|x| x + 1));
          }
        }
      }
    }

    head_depth
  }
}

#[test]
fn test() {
  dbg!(Order::from_iter([(0, 1, Relation::LT), (1, 2, Relation::LT), (2, 3, Relation::LT)]).complete());
}
