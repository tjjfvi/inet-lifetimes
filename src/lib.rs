#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return, const_option)]

use std::{
  collections::HashSet,
  fs,
  path::{Path, PathBuf},
};

use typed_arena::Arena;

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

use self::{parser::Parser, program::Program};

fn load(initial_path: impl AsRef<Path>) -> Result<Program, String> {
  let file_contents = Arena::<String>::new();
  let mut seen_files = HashSet::<PathBuf>::new();
  let mut parser = Parser::default();
  let mut todo_files = vec![initial_path.as_ref().canonicalize().unwrap()];
  while let Some(path) = todo_files.pop() {
    let file = file_contents.alloc(String::from_utf8(fs::read(&path).unwrap()).unwrap());
    parser.parse_file(file, |relative| {
      let path = path.parent().unwrap().join(relative).canonicalize().unwrap();
      if seen_files.insert(path.clone()) {
        todo_files.push(path);
      }
    })?;
  }

  Ok(parser.finish())
}

pub fn check(path: impl AsRef<Path>) -> Result<(), String> {
  let mut program: Program = load(path)?;
  program.check().report("check errors:").map_err(|x| x.to_string())?;
  Ok(())
}
