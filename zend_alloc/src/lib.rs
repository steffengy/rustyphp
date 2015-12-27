#![feature(allocator, libc)]
#![allocator]
#![no_std]

//! Zend allocator for Rust
//! Allows reusing pointers directly to PHP
//! without the need for reallocations

extern crate libc;
use libc::*;

// TODO: actually ensure to pass the feature zend_debug in a build step (from php_config somehow...)
// TODO: complete basic implementation (respect align better?)
//       https://github.com/rust-lang/rust/blob/master/src/liballoc_system/lib.rs


#[inline]
fn align_size(size: usize, align: usize) -> usize {
    // ((size + align - 1) & !(align - 1)) // from ZEND_MM_ALIGNED_SIZE_EX (does not work here)
    size + align + 1
}

#[cfg(feature = "zend_debug")]
pub mod debug_allocator {
    use libc::*;
    use super::align_size;

    extern "vectorcall" {
        fn _emalloc(size: size_t, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
        fn _erealloc(ptr: *mut c_void, size: size_t, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
        fn _efree(ptr: *mut c_void, filename: *const c_uchar, line: c_uint, orig_filename: *const c_uchar, orig_line: c_uint) -> *mut c_void;
    }

    #[no_mangle]
    pub extern fn __rust_allocate(size: usize, align: usize) -> *mut u8 {
        unsafe { _emalloc(align_size(size, align), file!().as_ptr(), line!(), file!().as_ptr(), line!()) as *mut _ }
    }

    #[no_mangle]
    pub extern fn __rust_deallocate(ptr: *mut u8, _old_size: usize, _align: usize) {
        unsafe { _efree(ptr as *mut _, file!().as_ptr(), line!(), file!().as_ptr(), line!()) };
    }

    #[no_mangle]
    pub extern fn __rust_reallocate(ptr: *mut u8, old_size: usize, size: usize, align: usize) -> *mut u8 {
        unsafe { _erealloc(ptr as *mut _, align_size(size, align), file!().as_ptr(), line!(), file!().as_ptr(), line!()) as *mut _ }
    }

    #[no_mangle]
    pub extern fn __rust_reallocate_inplace(ptr: *mut u8, old_size: usize,
                                            size: usize, align: usize) -> usize {
        old_size //TODO
    }

    #[no_mangle]
    pub extern fn __rust_usable_size(size: usize, align: usize) -> usize {
        size //TODO
    }
}
