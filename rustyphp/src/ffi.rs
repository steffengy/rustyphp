use libc::{c_long, c_uint};
use php_config::*;
use zend_module::*;
use super::types::*;

extern {
    pub fn zend_throw_exception(ce: *mut c_void, msg: *mut c_char, code: c_long);
    pub fn _zend_bailout(file: *mut c_char, line: u32);
    pub fn zend_register_internal_class_ex(ce: *mut ZendClassEntry, parent_ce: *mut ZendClassEntry) -> *mut ZendClassEntry;
    // pemalloc
    pub fn __zend_malloc(size: size_t) -> *mut c_void;
}

// TODO: debug/release definitions
extern {
    pub fn _array_init(arg: *mut Zval, size: u32, filename: *const c_uchar, line: c_uint) -> c_int;
}

/// TOOD: Is this also vectorcall on linux?
extern "vectorcall" {
    pub fn convert_to_long(op: *mut c_void);
    pub fn zend_hash_index_find(ht: *mut ZendArray, idx: zend_ulong) -> *mut Zval;
}

 //TODO debug/release definitions
extern "vectorcall" {
    pub fn _zval_dtor_func(ptr: *mut c_void, file: *mut c_char, line: u32);
    pub fn _emalloc(size: size_t, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
    pub fn _erealloc(ptr: *mut c_void, size: size_t, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
    pub fn _efree(ptr: *mut c_void, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
    pub fn _zend_hash_index_add_new(ht: *mut ZendArray, idx: zend_ulong, data: *mut Zval, filename: *const c_uchar, line: c_uint) -> *mut Zval;
}
