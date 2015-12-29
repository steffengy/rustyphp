#![feature(plugin, custom_attribute, const_fn)]
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
#[macro_export]
macro_rules! php_test {
    ($name:ident, $code:expr => $expect:expr) => {
        #[cfg(test)]
        #[test]
        fn $name() {
            // These uses are inside here, so that the cfg(test) still applies to them
            use std::path::Path;
            use std::process::Command;

            let target_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/debug/testext.dll"); //TODO path
            println!("{}", target_path.display());
            let output = Command::new(::rustyphp::testing::PHP_PATH)
                .arg(format!("-dextension=\"{}\"", target_path.display()))
                .args(&["-r", $code])
                .output()
                .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(output.status.success(), "exec fail: stdout: {} stderr: {}", stdout, stderr);
            assert!(stdout.trim() == $expect, "expected: \"{}\" got: \"{}\"", $expect, stdout.trim());
        }
    }
}
php_test!(test_hello_world, "var_dump(hello_world);" => "string(11) \"hello_world\"");
mod test_funcs;

// This has to be last (else it throws an compiler error "`php_func` cannot be used outside an extension" for test funcs)
php_ext!(
    name => "test_ext".as_ptr()
    version => "0.0.1".as_ptr()
);
