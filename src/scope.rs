use std::{
  collections::HashMap,
  ops::{Index, IndexMut},
};

use crate::{
  error::Error,
  index_vec::{Idx, IndexVec},
};

#[derive(Debug, Clone)]
pub struct Scope<K: Idx, T> {
  vec: IndexVec<K, Definition<T>>,
}

#[derive(Debug, Clone)]
struct Definition<T> {
  name: String,
  state: DefinitionState<T>,
}

#[derive(Debug, Clone)]
enum DefinitionState<T> {
  Undefined,
  Poisoned,
  Defined(T),
}

impl<K: Idx, T> Scope<K, T> {
  pub fn len(&self) -> K {
    self.vec.len()
  }

  pub fn get(&self, index: K) -> Result<&T, Error> {
    match &self.vec[index] {
      Definition { state: DefinitionState::Defined(value), .. } => Ok(value),
      Definition { name, state: DefinitionState::Undefined } => Err(Self::undefined_error(name)),
      Definition { name, state: DefinitionState::Poisoned } => Err(Self::poisoned_error(name)),
    }
  }

  pub fn poison(&mut self, index: K) {
    self.vec[index].state = DefinitionState::Poisoned;
  }

  pub fn push(&mut self, name: String, value: Option<T>) -> K {
    self.vec.push(Definition { name, state: value.map(DefinitionState::Defined).unwrap_or(DefinitionState::Undefined) })
  }

  pub fn name(&self, index: K) -> &str {
    &self.vec[index].name
  }

  pub fn poisoned(&self, index: K) -> bool {
    matches!(self.vec[index].state, DefinitionState::Poisoned)
  }

  pub fn iter(&self) -> impl Iterator<Item = (K, &str, &T)> {
    self.vec.iter().filter_map(|(k, Definition { name, state })| match state {
      DefinitionState::Defined(value) => Some((k, &**name, value)),
      _ => None,
    })
  }

  pub fn iter_mut(&mut self) -> impl Iterator<Item = (K, &str, &mut T)> {
    self.vec.iter_mut().filter_map(|(k, Definition { name, state })| match state {
      DefinitionState::Defined(value) => Some((k, &**name, value)),
      _ => None,
    })
  }

  pub fn expect_undefined(&self, index: K) -> Result<(), Error> {
    let def = &self.vec[index];
    if matches!(def.state, DefinitionState::Undefined) {
      Ok(())
    } else {
      Err(Error::from(format_args!("duplicate definition of {} `{}`", K::KIND, def.name)))
    }
  }

  pub fn try_define(&mut self, index: K, value: impl FnOnce() -> T) -> Result<(), Error> {
    if let Err(err) = self.expect_undefined(index) {
      self.poison(index);
      Err(err)
    } else {
      self.vec[index].state = DefinitionState::Defined(value());
      Ok(())
    }
  }

  pub fn or_define(&mut self, index: K, value: impl FnOnce() -> T) -> &mut T {
    match &mut self.vec[index].state {
      DefinitionState::Defined(value) => value,
      state => {
        *state = DefinitionState::Defined(value());
        let DefinitionState::Defined(value) = state else { unreachable!() };
        value
      }
    }
  }

  fn undefined_error(name: &str) -> Error {
    Error::from(format_args!("undefined {kind} `{name}`", kind = K::KIND))
  }

  fn poisoned_error(name: &str) -> Error {
    Error::from(format_args!("previous error in {kind} `{name}`", kind = K::KIND))
  }
}

pub struct ScopeBuilder<'i, K: Idx, T> {
  pub scope: Scope<K, T>,
  pub lookup: HashMap<&'i str, K>,
}

impl<'i, K: Idx, T> ScopeBuilder<'i, K, T> {
  pub fn get(&mut self, name: &'i str) -> K {
    *self.lookup.entry(name).or_insert_with(|| self.scope.push(name.to_owned(), None))
  }

  pub fn ensure_empty(&self) {
    debug_assert!(self.lookup.is_empty())
  }

  pub fn finish(&mut self) -> Scope<K, T> {
    self.lookup.clear();
    std::mem::take(&mut self.scope)
  }
}

impl<'i, K: Idx, T> Default for ScopeBuilder<'i, K, T> {
  fn default() -> Self {
    Self { scope: Default::default(), lookup: Default::default() }
  }
}

impl<K: Idx, T> Index<K> for Scope<K, T> {
  type Output = T;

  fn index(&self, index: K) -> &Self::Output {
    match &self.vec[index].state {
      DefinitionState::Defined(value) => value,
      _ => unreachable!(),
    }
  }
}

impl<K: Idx, T> IndexMut<K> for Scope<K, T> {
  fn index_mut(&mut self, index: K) -> &mut Self::Output {
    match &mut self.vec[index].state {
      DefinitionState::Defined(value) => value,
      _ => unreachable!(),
    }
  }
}

impl<K: Idx, T> Default for Scope<K, T> {
  fn default() -> Self {
    Self { vec: Default::default() }
  }
}
