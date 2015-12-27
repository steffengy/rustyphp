use libc::{c_void, c_char, c_long};
use super::zval::*;

extern {
    pub fn zend_throw_exception(ce: *mut c_void, msg: *mut c_char, code: c_long);
    pub fn _zend_bailout(file: *mut c_char, line: u32);
}

/// TOOD: Is this also vectorcall on linux?
extern "vectorcall" {
    pub fn convert_to_long(op: *mut c_void);
}

macro_rules! convert_zval {
    ($conversion_func:ident, $zv:expr) => {
        unsafe { $crate::external::$conversion_func($zv as *const _ as *mut _); }
    }
}
