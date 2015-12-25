Rustyphp
==================
RustyPHP allows building PHP extensions with Rust.

An example is available in the [`example`](example/src/lib.rs) subfolder.

Building Instructions
==================
1. Gather information about your PHP installation using [cfg_builder](cfg_builder/README.md) which gives you something like
```rust
use rustyphp::types::{c_int, c_uchar};

pub static ZEND_MODULE_API_NO: c_int = 20151012;
pub static ZEND_MODULE_BUILD_ID: &'static str = "API20151012,TS,debug,VC14";
pub static ZEND_ZTS: c_uchar = 1;
pub static ZEND_DEBUG: c_uchar = 1;
/// zend_long
#[allow(non_camel_case_types)]
pub type zend_long = i64;
/// zend_double
#[allow(non_camel_case_types)]
pub type zend_double = f64;
```
which you need to include in your extensions by for example copying it into the same source tree and using
```rust
mod php_config;
use php_config::*;
```

2. Build it using cargo and load the resulting dylib (dll/so)

