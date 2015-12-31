#![feature(placement_new_protocol, placement_in_syntax)]
extern crate libc;

pub mod php_config;
pub use php_config::*;

/// until we have an own result type
pub use std::result;

#[macro_use]
pub mod macros;

pub mod zend_mm;
pub use zend_mm::*;
pub mod types;
pub use types::*;

pub mod ffi;

// keep this last before testing
pub mod zend_module;
pub use zend_module::*;

#[cfg(any(feature = "test",test))]
pub mod testing {
    include!(concat!(env!("OUT_DIR"), "/test_helper.rs"));
}

#[cfg(any(feature = "test",test))]
pub use testing::*;
