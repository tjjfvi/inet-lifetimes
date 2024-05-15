use insta::{assert_snapshot, Settings};

use inet_lifetimes::check;

const OK_PATHS: &[&str] = &["examples/fn.inlt", "examples/list.inlt", "examples/nat.inlt"];

const ERR_PATHS: &[&str] = &["tests/programs/bad.inlt"];

#[test]
fn test_ok() {
  let mut settings = Settings::new();
  let mut err_count = 0;
  for path in OK_PATHS {
    settings.set_input_file(path);
    let result = check(path);
    if let Err(err) = result {
      println!("{path} failed: {err}");
      err_count += 1;
    }
  }
  if err_count != 0 {
    panic!("{err_count} failed")
  }
}

#[test]
fn test_err() {
  let mut settings = Settings::new();
  for &path in ERR_PATHS {
    settings.set_prepend_module_to_snapshot(false);
    settings.set_omit_expression(true);
    settings.set_input_file(path);
    let result = check(path).err().unwrap_or("no errors".to_owned());
    settings.bind(|| {
      assert_snapshot!(path, result);
    });
  }
}
