#![feature(plugin, custom_attribute, const_fn, asm)]
#![plugin(rustyphp_plugin)]

#[macro_use]
extern crate rustyphp;

use rustyphp::*;

/// Sample PHP function printing hello world, replace by macro
/// void zif_hello_world(zend_execute_data *execute_data, zval *return_value)
#[php_func]
fn hello_world() {
    println!("hello world");
}

// Test related stuff:
// Allow testing the integration of the built .dlls into PHP
// This requires `cargo build` to be run before
macro_rules! php_test_helper {
    (expect, $v:expr) => (php_test_helper!(SOME, $v));
    (code, $v:expr) => (php_test_helper!(SOME, $v));
    (check_func, $v:expr) => (Box::new($v));
    (SOME, $v:expr) => (Some($v));
    ($k:ident, $v:expr) => ($v);
}

#[macro_export]
macro_rules! php_test {
    ($name:ident, $($k:ident => $v:expr),*) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            // These uses are inside here, so that the cfg(test) still applies to them
            use std::path::Path;
            use std::process::Command;

            struct Settings<'a> {
                check_func: Box<Fn(&str, &str, &str)>,
                code: Option<&'a str>,
                status_success: bool,
                expect: Option<&'a str>
            }

            let mut settings = Settings {
                check_func: Box::new(|expect: &str, stdout: &str, _| assert!(stdout.trim() == expect, "EXPECTED:\n{}\nGOT:{}", expect, stdout.trim())),
                code: None,
                status_success: true,
                expect: None
            };
            $(
                settings.$k = php_test_helper!($k, $v);
            )*

            let target_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/testext.dll"); //TODO path
            println!("{}", target_path.display());
            let output = Command::new(::rustyphp::testing::PHP_PATH)
                .arg(format!("-dextension=\"{}\"", target_path.display()))
                .args(&["-r", settings.code.unwrap()])
                .output()
                .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(!settings.status_success || output.status.success(), "ERR: \nstdout: {}\n stderr: {}", stdout, stderr);
            if settings.expect.is_some() {
                (settings.check_func)(settings.expect.unwrap(), &stdout, &stderr)
            }
        }
    }
}
php_test!(test_hello_world, code => "var_dump(hello_world);", expect => "string(11) \"hello_world\"");
mod test_funcs;

// This has to be last (else it throws an compiler error "`php_func` cannot be used outside an extension" for test funcs)
php_ext!(
    name => "test_ext".as_ptr()
    version => "0.0.1".as_ptr()
);
