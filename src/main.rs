use std::{env, process::ExitCode};

use inet_lifetimes::check;

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
