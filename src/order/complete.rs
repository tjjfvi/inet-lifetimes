use nohash_hasher::IntMap;

use crate::index_vec::Idx;

use super::{Flag, Order};

impl<I: Idx> Order<I> {
  pub fn complete(&self) -> Order<I> {
    self.clear_flags();
    let mut completionist =
      Completionist { source: self, output: Order::default(), interesting: I::from(0), cutoff: I::from(usize::MAX) };
    for (a, _) in &self.els {
      completionist.visit(a, Some(0));
    }
    completionist.output
  }

  pub fn difference(&self, other: &Order<I>) -> Order<I> {
    other.clear_flags();
    let mut completionist =
      Completionist { source: other, output: Order::default(), interesting: I::from(0), cutoff: I::from(usize::MAX) };

    let mut diff = Order::default();
    for (a, b, eq) in self.iter() {
      if !other.has(a, b, eq) {
        completionist.visit(a, Some(0));
        if !completionist.output.has(a, b, eq) {
          diff.relate_lt(a, b, eq)
        }
      }
    }

    diff
  }

  pub fn truncate(&self, cutoff: I) -> Order<I> {
    self.clear_flags();
    let mut completionist = Completionist { source: self, output: Order::default(), interesting: cutoff, cutoff };
    for (a, _) in self.els.iter().take(cutoff.into()) {
      completionist.visit(a, Some(0));
    }
    completionist.output.els.truncate(cutoff);
    completionist.output
  }

  fn has(&self, a: I, b: I, eq: bool) -> bool {
    self.els.get(a).and_then(|x| x.rels.get(&b)).is_some_and(|self_eq| !self_eq || eq)
  }
}

struct Completionist<'a, I: Idx> {
  source: &'a Order<I>,
  output: Order<I>,
  interesting: I,
  cutoff: I,
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

    for (&b, _) in &el.rels {
      if b >= self.interesting {
        head_depth = head_depth.min(self.visit(b, depth.map(|x| x + 1)));
      }
    }

    if depth.is_some_and(|x| x > head_depth) {
      el.flag.set(Flag::Cycle(head_depth));
    } else {
      let mut rels = IntMap::default();
      for (&b, &eq_0) in &el.rels {
        if b < self.cutoff {
          rels.insert(b, eq_0);
        }
        if b >= self.interesting {
          if let Some(other) = self.output.els.get(b) {
            for (&c, &eq_1) in &other.rels {
              rels.insert(c, eq_0 && eq_1);
            }
          }
        }
      }
      self.output.els.get_or_extend(a).rels = rels;

      el.flag.set(Flag::Done);
      if depth == Some(head_depth) {
        for (&b, _) in &el.rels {
          self.visit(b, depth.map(|x| x + 1));
        }
      }
    }

    head_depth
  }
}

#[test]
fn test() {
  dbg!(Order::from_iter([(0, 1, false), (1, 2, false), (2, 3, false)]).complete());
}
