use std::fmt::Display;

use crate::{
  index_vec::Idx,
  util::{Captures, DisplayFn},
};

use super::Order;

impl<I: Idx> Order<I> {
  pub fn find_cycles(&self) -> Vec<Vec<I>> {
    for el in self.els.values() {
      el.flag.take();
    }
    let mut finder =
      CycleFinder { order: self, finished_cycles: vec![], active_cycles_0: vec![], active_cycles_1: vec![] };
    for (a, _) in &self.els {
      finder.visit(a, 0);
    }
    debug_assert!(finder.active_cycles_0.len() == 0);
    debug_assert!(finder.active_cycles_1.len() == 0);
    finder.finished_cycles
  }

  pub fn cycle_error<D: Display>(
    &self,
    cycles: Vec<Vec<I>>,
    base_message: &str,
    display_item: impl Fn(I) -> D,
  ) -> Result<(), String> {
    if !cycles.is_empty() {
      use std::fmt::Write;
      let mut error = base_message.to_owned();
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
          let op = if self.els[a].rels[&b] { "<=" } else { "<" };
          write!(f, " {op} ")?;
        }
        write!(f, "{}", display_item(b))?;
        last = Some(b);
      }
      Ok(())
    })
  }
}

struct CycleFinder<'a, I: Idx> {
  order: &'a Order<I>,
  finished_cycles: Vec<Vec<I>>,
  active_cycles_0: Vec<Vec<I>>,
  active_cycles_1: Vec<Vec<I>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CycleFlag {
  #[default]
  None,
  Visiting {
    strong_depth: usize,
  },
  Visited,
}

impl<'a, I: Idx> CycleFinder<'a, I> {
  fn visit(&mut self, a: I, strong_depth: usize) {
    let Some(el) = &self.order.els.get(a) else { return };
    match el.flag.get() {
      CycleFlag::Visited => return,
      CycleFlag::Visiting { strong_depth: previous_depth } => {
        if strong_depth > previous_depth {
          self.active_cycles_0.push(vec![a]);
        }
        return;
      }
      _ => {}
    }
    el.flag.set(CycleFlag::Visiting { strong_depth });
    std::mem::swap(&mut self.active_cycles_0, &mut self.active_cycles_1);
    let new_cycles_start = self.active_cycles_0.len();
    for (&b, &eq) in &el.rels {
      self.visit(b, if eq { strong_depth } else { strong_depth + 1 })
    }
    for mut cycle in self.active_cycles_0.drain(new_cycles_start..) {
      cycle.push(a);
      if cycle[0] == a {
        cycle.reverse();
        self.finished_cycles.push(cycle);
      } else {
        self.active_cycles_1.push(cycle);
      }
    }
    el.flag.set(CycleFlag::Visited);
    std::mem::swap(&mut self.active_cycles_0, &mut self.active_cycles_1);
  }
}

#[test]
fn test_simple_cases() {
  for (order, cycles) in [
    (Order::default(), vec![]),
    (Order::from_iter([(0, 1, false), (1, 2, false), (2, 3, false)]), vec![]),
    (Order::from_iter([(0, 1, false), (1, 2, false), (0, 2, false)]), vec![]),
    (Order::from_iter([(0, 1, true), (1, 3, true), (0, 2, false), (2, 3, true), (3, 4, false)]), vec![]),
    (Order::from_iter([(0, 1, true), (1, 0, true)]), vec![]),
    (Order::from_iter([(0, 0, true)]), vec![]),
    //
    (Order::from_iter([(0, 1, false), (1, 0, true)]), vec![vec![0, 1, 0]]),
    (Order::from_iter([(0, 0, false)]), vec![vec![0, 0]]),
    (Order::from_iter([(0, 1, false), (0, 2, true), (1, 0, true), (2, 0, false)]), vec![vec![0, 1, 0], vec![0, 2, 0]]),
    (
      Order::from_iter([(0, 1, false), (0, 2, true), (1, 0, true), (2, 0, false), (1, 2, false)]),
      vec![vec![0, 1, 0], vec![0, 1, 2, 0]],
    ),
    (
      Order::from_iter([(0, 1, false), (0, 2, true), (1, 0, true), (2, 0, false), (2, 1, false)]),
      vec![vec![0, 1, 0], vec![0, 2, 0]],
    ),
  ] {
    assert_eq!(order.find_cycles(), cycles, "order {:?}", order);
  }
}

// We intentionally don't report *every* possible cycle, as this could take, in
// the worst case, exponential time.
//
// However, we do ensure that every node that is involved in a cycle is involved
// in at least one reported cycle.
#[test]
fn test_extreme_cases() {
  for n in 1..=100 {
    // For n=3 this graph looks like:
    // ```text
    // a---b---c---a
    //  \ / \ / \ /
    //   X   X   X
    //  / \ / \ / \
    // x---y---z---x
    // ```
    // where two connected nodes denotes a less-than relation flowing
    // left-to-right (note the duplication of `a` and `x`).
    //
    // This has an number of possible cycles exponential on `n`.
    //
    // We only report a linear number of these cycles.

    let m = n * 2;
    let order = Order::from_iter((0..m).flat_map(|x| [(x, (x + 2) % m, false), (x, ((x + 2) % m) ^ 1, false)]));
    let cycles = order.find_cycles();

    assert_eq!(cycles.len(), n + 2);

    assert!(every_node_represented(&order, &cycles));
  }

  for n in 1..=20 {
    // This order requires `a < b` for all `a`, `b`.
    let order = Order::from_iter((0..n).flat_map(|a| (0..n).map(move |b| (a, b, false))));
    let cycles = order.find_cycles();

    // Quadratic on `n`; linear on edge count.
    assert_eq!(cycles.len(), n * (n + 1) / 2);

    assert!(every_node_represented(&order, &cycles));
  }

  fn every_node_represented(order: &Order<usize>, cycles: &Vec<Vec<usize>>) -> bool {
    order.els.iter().all(|(a, _)| cycles.iter().any(|cycle| cycle.contains(&a)))
  }
}
