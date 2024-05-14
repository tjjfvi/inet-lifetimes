#![feature(impl_trait_in_assoc_type, impl_trait_in_fn_trait_return, const_option, let_chains)]

use std::{env, fs, process::ExitCode};

use program::Program;

mod globals;
mod index_vec;
mod lifetimes;
mod order;
mod parser;
mod program;
mod util;
mod vars;

fn main() -> ExitCode {
  let mut any = false;
  let mut code = ExitCode::SUCCESS;
  for path in env::args().skip(1) {
    any = true;
    if let Err(e) = check(&path) {
      println!("{path}:\n\n{}\n\n", e);
      code = ExitCode::FAILURE;
    } else {
      println!("{path}: ok")
    }
  }
  if !any {
    println!("supply a path");
    code = ExitCode::FAILURE;
  }
  code
}

fn check(path: &str) -> Result<(), String> {
  let src = String::from_utf8(fs::read(path).unwrap()).unwrap();
  let mut program: Program = src.parse()?;
  program.check()?;
  Ok(())
}
