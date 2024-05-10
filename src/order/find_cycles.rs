use crate::index_vec::Idx;

use super::{Flag, Order};

impl<I: Idx> Order<I> {
  pub fn find_cycles(&self) -> Vec<Vec<I>> {
    self.clear_flags();
    let mut finder =
      CycleFinder { order: self, finished_cycles: vec![], active_cycles_0: vec![], active_cycles_1: vec![] };
    for (a, _) in &self.els {
      finder.visit(a, 0);
    }
    debug_assert!(finder.active_cycles_0.len() == 0);
    debug_assert!(finder.active_cycles_1.len() == 0);
    finder.finished_cycles
  }
}

struct CycleFinder<'a, I: Idx> {
  order: &'a Order<I>,
  finished_cycles: Vec<Vec<I>>,
  active_cycles_0: Vec<Vec<I>>,
  active_cycles_1: Vec<Vec<I>>,
}

impl<'a, I: Idx> CycleFinder<'a, I> {
  fn visit(&mut self, a: I, strong_depth: usize) {
    let Some(el) = &self.order.els.get(a) else { return };
    match el.flag.get() {
      Flag::Done => return,
      Flag::Cycle(previous_depth) => {
        if strong_depth > previous_depth {
          self.active_cycles_0.push(vec![a]);
        }
        return;
      }
      _ => {}
    }
    el.flag.set(Flag::Cycle(strong_depth));
    std::mem::swap(&mut self.active_cycles_0, &mut self.active_cycles_1);
    let new_cycles_start = self.active_cycles_0.len();
    for (&b, &rel) in &el.rels {
      if let Some(rel) = rel.forward_component() {
        self.visit(b, if rel.allows_equal() { strong_depth } else { strong_depth + 1 })
      }
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
    el.flag.set(Flag::Done);
    std::mem::swap(&mut self.active_cycles_0, &mut self.active_cycles_1);
  }
}

#[cfg(test)]
mod tests {
  use crate::order::{Order, Relation};

  const LE: Relation = Relation::LE;
  const LT: Relation = Relation::LT;

  #[test]
  fn test_simple_cases() {
    for (order, cycles) in [
      (Order::default(), vec![]),
      (Order::from_iter([(0, 1, LT), (1, 2, LT), (2, 3, LT)]), vec![]),
      (Order::from_iter([(0, 1, LT), (1, 2, LT), (0, 2, LT)]), vec![]),
      (Order::from_iter([(0, 1, LE), (1, 3, LE), (0, 2, LT), (2, 3, LE), (3, 4, LT)]), vec![]),
      (Order::from_iter([(0, 1, LE), (1, 0, LE)]), vec![]),
      (Order::from_iter([(0, 0, LE)]), vec![]),
      //
      (Order::from_iter([(0, 1, LT), (1, 0, LE)]), vec![vec![0, 1, 0]]),
      (Order::from_iter([(0, 0, LT)]), vec![vec![0, 0]]),
      (Order::from_iter([(0, 1, LT), (0, 2, LE), (1, 0, LE), (2, 0, LT)]), vec![vec![0, 1, 0], vec![0, 2, 0]]),
      (
        Order::from_iter([(0, 1, LT), (0, 2, LE), (1, 0, LE), (2, 0, LT), (1, 2, LT)]),
        vec![vec![0, 1, 0], vec![0, 1, 2, 0]],
      ),
      (
        Order::from_iter([(0, 1, LT), (0, 2, LE), (1, 0, LE), (2, 0, LT), (2, 1, LT)]),
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
      let order = Order::from_iter((0..m).flat_map(|x| [(x, (x + 2) % m, LT), (x, ((x + 2) % m) ^ 1, LT)]));
      let cycles = order.find_cycles();

      assert_eq!(cycles.len(), n + 2);

      assert!(every_node_represented(&order, &cycles));
    }

    for n in 1..=20 {
      // This order requires `a < b` for all `a`, `b`.
      let order = Order::from_iter((0..n).flat_map(|a| (0..n).map(move |b| (a, b, LT))));
      let cycles = order.find_cycles();

      // Quadratic on `n`; linear on edge count.
      assert_eq!(cycles.len(), n * (n + 1) / 2);

      assert!(every_node_represented(&order, &cycles));
    }

    fn every_node_represented(order: &Order<usize>, cycles: &Vec<Vec<usize>>) -> bool {
      order.els.iter().all(|(a, _)| cycles.iter().any(|cycle| cycle.contains(&a)))
    }
  }
}
