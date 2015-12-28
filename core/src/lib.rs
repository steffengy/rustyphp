#![feature(heap_api)]
#![feature(thread_local_state)]
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

