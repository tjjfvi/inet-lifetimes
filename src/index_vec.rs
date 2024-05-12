use std::{
  fmt::Debug,
  hash::Hash,
  marker::PhantomData,
  ops::{Index, IndexMut},
};

use nohash_hasher::IsEnabled;

pub trait Idx: Copy + Eq + Ord + Hash + IsEnabled + From<usize> + Into<usize> + Debug {}

impl Idx for usize {}

#[macro_export]
macro_rules! new_index {
  ($vis:vis $Index:ident) => {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    $vis struct $Index(pub usize);

    impl From<usize> for $Index {
      fn from(index: usize) -> Self {
        Self(index)
      }
    }

    impl Into<usize> for $Index {
      fn into(self) -> usize {
        self.0
      }
    }

    impl $crate::index_vec::Idx for $Index {}
    impl nohash_hasher::IsEnabled for $Index {}
  };
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexVec<I: Idx, T> {
  vec: Vec<T>,
  __: PhantomData<fn(&I)>,
}

impl<I: Idx, T> IndexVec<I, T> {
  pub fn push(&mut self, value: T) -> I {
    let index = I::from(self.vec.len());
    self.vec.push(value);
    index
  }
  pub fn len(&self) -> I {
    I::from(self.vec.len())
  }
  pub fn truncate(&mut self, max_len: I) {
    self.vec.truncate(max_len.into())
  }
  pub fn get(&self, index: I) -> Option<&T> {
    self.vec.get(index.into())
  }
  pub fn get_mut(&mut self, index: I) -> Option<&mut T> {
    self.vec.get_mut(index.into())
  }
  pub fn into_iter(self) -> impl Iterator<Item = (I, T)> {
    self.vec.into_iter().enumerate().map(|(index, value)| (index.into(), value))
  }
  pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
    self.vec.iter().enumerate().map(|(index, value)| (index.into(), value))
  }
  pub fn iter_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> {
    self.vec.iter_mut().enumerate().map(|(index, value)| (index.into(), value))
  }
  pub fn keys(&self) -> impl Iterator<Item = I> {
    (0..self.vec.len()).map(I::from)
  }
  pub fn values(&self) -> impl Iterator<Item = &T> {
    self.vec.iter()
  }
  pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
    self.vec.iter_mut()
  }
  pub fn get_or_extend(&mut self, index: I) -> &mut T
  where
    T: Default,
  {
    let index: usize = index.into();
    if index >= self.vec.len() {
      self.vec.resize_with(index + 1, T::default);
    }
    &mut self.vec[index]
  }
}

impl<I: Idx, T> Index<I> for IndexVec<I, T> {
  type Output = T;

  fn index(&self, index: I) -> &Self::Output {
    &self.vec[index.into()]
  }
}

impl<I: Idx, T> IndexMut<I> for IndexVec<I, T> {
  fn index_mut(&mut self, index: I) -> &mut Self::Output {
    &mut self.vec[index.into()]
  }
}

impl<I: Idx, T> IntoIterator for IndexVec<I, T> {
  type Item = (I, T);
  type IntoIter = impl Iterator<Item = (I, T)>;

  fn into_iter(self) -> Self::IntoIter {
    self.into_iter()
  }
}

impl<'a, I: Idx, T> IntoIterator for &'a IndexVec<I, T> {
  type Item = (I, &'a T);
  type IntoIter = impl Iterator<Item = (I, &'a T)>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter()
  }
}

impl<'a, I: Idx, T> IntoIterator for &'a mut IndexVec<I, T> {
  type Item = (I, &'a mut T);
  type IntoIter = impl Iterator<Item = (I, &'a mut T)>;

  fn into_iter(self) -> Self::IntoIter {
    self.iter_mut()
  }
}

impl<I: Idx + Debug, T: Debug> Debug for IndexVec<I, T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut f = f.debug_map();
    f.entries(self);
    f.finish()
  }
}

impl<I: Idx, T> From<Vec<T>> for IndexVec<I, T> {
  fn from(vec: Vec<T>) -> Self {
    IndexVec { vec, __: PhantomData }
  }
}

impl<I: Idx, T> From<IndexVec<I, T>> for Vec<T> {
  fn from(value: IndexVec<I, T>) -> Self {
    value.vec
  }
}

impl<I: Idx, T> Default for IndexVec<I, T> {
  fn default() -> Self {
    Self { vec: Default::default(), __: Default::default() }
  }
}
