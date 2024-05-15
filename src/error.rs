use std::fmt::{self, Display, Write};

use crate::util::Captures;

pub struct Error(String, ErrorGroup);

impl Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    self.fmt(f, 1)
  }
}

impl Error {
  fn fmt(&self, f: &mut fmt::Formatter<'_>, indent: usize) -> fmt::Result {
    f.write_str(&self.0)?;
    for suberror in &self.1 .0 {
      f.write_char('\n')?;
      for _ in 0..indent {
        f.write_str("  ")?;
      }
      suberror.fmt(f, indent + 1)?;
    }
    Ok(())
  }

  pub fn context<'a>(ctx: impl Display + 'a) -> impl (FnOnce(Error) -> Error) + Captures<&'a ()> {
    move |err| Error(ctx.to_string(), ErrorGroup(vec![err]))
  }
}

#[derive(Default)]
pub struct ErrorGroup(Vec<Error>);

pub trait Pushable {
  type Output;
  fn push_to(self, group: &mut ErrorGroup) -> Self::Output;
}

impl<T> Pushable for Result<T, Error> {
  type Output = Option<T>;

  fn push_to(self, group: &mut ErrorGroup) -> Self::Output {
    match self {
      Ok(value) => Some(value),
      Err(err) => {
        group.push(err);
        None
      }
    }
  }
}

impl Pushable for Error {
  type Output = ();

  fn push_to(self, group: &mut ErrorGroup) -> Self::Output {
    group.0.push(self);
  }
}

impl Pushable for ErrorGroup {
  type Output = ();

  fn push_to(self, group: &mut ErrorGroup) -> Self::Output {
    group.0.extend(self.0);
  }
}

impl ErrorGroup {
  pub fn push<T: Pushable>(&mut self, value: T) -> T::Output {
    value.push_to(self)
  }

  pub fn report(&mut self, label: impl Display) -> Result<(), Error> {
    if self.0.is_empty() {
      Ok(())
    } else {
      Err(Error(label.to_string(), std::mem::take(self)))
    }
  }
}

pub trait ToError: Display {}

impl ToError for fmt::Arguments<'_> {}
impl<T: Display> ToError for &'_ T {}
impl<T: Display> ToError for &'_ mut T {}

impl<T: ToError> From<T> for Error {
  fn from(value: T) -> Self {
    Error::from(value.to_string())
  }
}

impl From<String> for Error {
  fn from(value: String) -> Self {
    Error(value, ErrorGroup::default())
  }
}

#[macro_export]
macro_rules! err {
  ($($x:tt)*) => {
    $crate::error::Error::from(format_args!($($x)*))
  };
}
