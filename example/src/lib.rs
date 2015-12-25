#![feature(plugin, custom_attribute)]
#![plugin(rustyphp_plugin)]

#[macro_use]
extern crate rustyphp;

use rustyphp::types::*;

/// Sample PHP function printing hello world, replace by macro
/// void zif_hello_world(zend_execute_data *execute_data, zval *return_value)
#[php_func]
fn hello_world() -> Option<i32> {
    println!("hello_world_from_rust");
    Some(16)
}

extern fn on_startup(_: c_int, _: c_int) -> c_int {
    println!("startup");
    1
}

php_ext!(
    name => "example_ext".as_ptr()
    version => "0.0.1".as_ptr()
    module_startup_func => Some(on_startup)
);
