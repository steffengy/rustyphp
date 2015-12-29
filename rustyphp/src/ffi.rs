use libc::{c_long, c_uint};
use super::types::*;

extern {
    pub fn zend_throw_exception(ce: *mut c_void, msg: *mut c_char, code: c_long);
    pub fn _zend_bailout(file: *mut c_char, line: u32);
}

/// TOOD: Is this also vectorcall on linux?
extern "vectorcall" {
    pub fn convert_to_long(op: *mut c_void);
}

 //TODO debug/release definitions
extern "vectorcall" {
    pub fn _zval_dtor_func(ptr: *mut c_void, file: *mut c_char, line: u32);
    pub fn _emalloc(size: size_t, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
    pub fn _erealloc(ptr: *mut c_void, size: size_t, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
    pub fn _efree(ptr: *mut c_void, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
}
