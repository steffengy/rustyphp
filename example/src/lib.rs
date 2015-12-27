#![feature(plugin, custom_attribute)]
#![plugin(rustyphp_plugin)]

#[macro_use]
extern crate rustyphp;

use rustyphp::*;
use rustyphp::types::*;

/// Sample PHP function printing hello world, replace by macro
/// void zif_hello_world(zend_execute_data *execute_data, zval *return_value)
#[php_func]
fn hello_world(p1: u16) -> Option<bool> {
    println!("hello_world_from_rus {}", p1);
    throw_exception!("test");
    Some(true)
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
