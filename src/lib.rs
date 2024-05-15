#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return, const_option)]

use std::fs;

mod error;
mod globals;
mod index_vec;
mod lifetimes;
mod order;
mod parser;
mod program;
mod scope;
mod util;
mod vars;

use program::Program;

pub fn check(path: &str) -> Result<(), String> {
  let src = String::from_utf8(fs::read(path).unwrap()).unwrap();
  let mut program: Program = src.parse()?;
  program.check().report("errors:").map_err(|x| x.to_string())?;
  Ok(())
}
