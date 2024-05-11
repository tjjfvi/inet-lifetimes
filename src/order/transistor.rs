use crate::index_vec::Idx;

use super::{Flag, Order, Relation};

impl<I: Idx> Order<I> {
  fn complete(&self) -> Transistor<I> {
    Transistor::new(
      self,
      TransistorConfig {
        enter: &|_, rel, _| rel.forward_component().is_some(),
        remap: &|_, rel, _| rel.forward_component(),
        trans: &|_, r0, _, r1, _| Some(r0.forward_component()? & r1.forward_component()?),
      },
    )
  }

  pub fn difference(&self, other: &Order<I>) -> Order<I> {
    let mut transistor = other.complete();

    let mut diff = Order::default();
    for (a, b, rel) in self.iter_forward() {
      if !other.has(a, b, rel) {
        transistor.visit(a);
        if !transistor.output.has(a, b, rel) {
          diff.relate(a, b, rel)
        }
      }
    }

    diff
  }

  pub fn omit(&self, omit: &dyn Fn(I) -> bool) -> Order<I> {
    let mut output = Transistor::new(
      self,
      TransistorConfig {
        enter: &|_, rel, b| omit(b) && rel.forward_component().is_some(),
        remap: &|_, rel, b| (!omit(b)).then(|| rel.forward_component()).flatten(),
        trans: &|_, r0, _, r1, _| Some(r0.forward_component()? & r1.forward_component()?),
      },
    )
    .finish_where(|a| !omit(a));
    for (a, el) in &mut output.els {
      if omit(a) {
        *el = Default::default();
      }
    }
    output
  }

  fn has(&self, a: I, b: I, rel: Relation) -> bool {
    self.els.get(a).and_then(|x| x.rels.get(&b)).is_some_and(|&has_rel| (has_rel & rel) == has_rel)
  }
}

pub struct TransistorConfig<'a, I: Idx> {
  pub enter: &'a dyn Fn(I, Relation, I) -> bool,
  pub remap: &'a dyn Fn(I, Relation, I) -> Option<Relation>,
  pub trans: &'a dyn Fn(I, Relation, I, Relation, I) -> Option<Relation>,
}

pub struct Transistor<'a, I: Idx> {
  source: &'a Order<I>,
  pub output: Order<I>,
  cfg: TransistorConfig<'a, I>,
}

impl<'a, I: Idx> Transistor<'a, I> {
  pub fn new(source: &'a Order<I>, cfg: TransistorConfig<'a, I>) -> Self {
    source.clear_flags();
    Transistor { source, output: Order::default(), cfg }
  }

  #[allow(unused)]
  pub fn finish(self) -> Order<I> {
    self.finish_where(|_| true)
  }

  pub fn finish_where(mut self, visit: impl Fn(I) -> bool) -> Order<I> {
    self.visit_where(visit);
    self.output
  }

  pub fn visit_where(&mut self, visit: impl Fn(I) -> bool) {
    for (a, _) in &self.source.els {
      if visit(a) {
        self.visit(a);
      }
    }
  }

  pub fn visit(&mut self, a: I) {
    self._visit(a, Some(0));
  }

  fn _visit(&mut self, a: I, depth: Option<usize>) -> usize {
    let Some(el) = self.source.els.get(a) else { return usize::MAX };
    match el.flag.get() {
      Flag::None => el.flag.set(Flag::Cycle(depth.unwrap())),
      Flag::Done => return usize::MAX,
      Flag::Cycle(d) if depth.is_some() => return d,
      Flag::Cycle(_) => el.flag.set(Flag::Done),
    }

    let mut head_depth = usize::MAX;

    for (&b, &rel) in &el.rels {
      if (self.cfg.enter)(a, rel, b) {
        head_depth = head_depth.min(self._visit(b, depth.map(|x| x + 1)));
      }
    }

    for (&b, &rel_ab) in &el.rels {
      if let Some(new_rel) = (self.cfg.remap)(a, rel_ab, b) {
        self.output.relate(a, b, new_rel);
      }
      if a == b {
        continue;
      }
      if (self.cfg.enter)(a, rel_ab, b) {
        if let Some(other) = self.output.els.get_mut(b) {
          let rels = std::mem::take(&mut other.rels);
          for (&c, &rel_bc) in &rels {
            if b == c {
              continue;
            }
            if let Some(rel_ac) = (self.cfg.trans)(a, rel_ab, b, rel_bc, c) {
              self.output.relate(a, c, rel_ac);
            }
          }
          self.output.els[b].rels = rels;
        }
      }
    }

    if depth.is_some_and(|x| x > head_depth) {
      el.flag.set(Flag::Cycle(head_depth));
    } else {
      el.flag.set(Flag::Done);
      if depth == Some(head_depth) {
        for (&b, &rel) in &el.rels {
          if (self.cfg.enter)(a, rel, b) {
            self._visit(b, None);
          }
        }
      }
    }

    head_depth
  }
}

#[test]
fn test_complete() {
  let order = Order::from_iter([(0, 1, Relation::LT), (1, 2, Relation::LT), (2, 3, Relation::LT)]);
  for super_enter in [false, true] {
    let mut transistor = order.complete();
    if super_enter {
      transistor.cfg.enter = &|_, _, _| true;
    }
    assert_eq!(
      format!("{:?}", transistor.finish()),
      format!(
        "{:?}",
        Order::from_iter([
          (0, 1, Relation::LT),
          (0, 2, Relation::LT),
          (0, 3, Relation::LT),
          (1, 2, Relation::LT),
          (1, 3, Relation::LT),
          (2, 3, Relation::LT)
        ])
      ),
    );
  }
}
