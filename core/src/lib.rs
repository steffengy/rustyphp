#![feature(heap_api, alloc)]
extern crate alloc;
extern crate libc;

pub mod types;

#[macro_use]
pub mod external;
pub mod zval;

use types::*;
use external::*;
pub use zval::*;

pub mod php_config;
pub use php_config::*;
