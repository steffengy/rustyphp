//! zend_array/Hastable related stuff
use std::any::Any;
use std::mem;
use std::ops::{Index, IndexMut};
use php_config::*;
use types::*;
use ffi;
use zend_mm::{Refcounted, ZendRefcounted};
use zstr::CZendString;

#[derive(Debug)]
#[repr(C)]
pub struct ZendArray {
    refc: ZendRefcounted,
    /// flags, nApplyCount, nIteratorsCount, reserve (all: LE as u8)
    flags: u32,
    table_mask: u32,
    ar_data: *mut ZendBucket,
    num_used: u32,
    num_elems: u32,
    table_size: u32,
    inter_ptr: u32,
    next_free_el: zend_long,
    dtor_func: extern "C" fn(*mut Zval)
}

impl ZendArray {
    /// Initialize the returned array after by either passing it into zend_hash_init
    /// or by passing the zval into _array_init
    pub fn new() -> Refcounted<ZendArray> {
        let arr: ZendArray = unsafe { mem::uninitialized() };
        Refcounted::new(arr)
    }
}

impl<'a> ZendArray {
    pub fn get<T>(&self, idx: zend_ulong) -> Result<T, String> where Result<T, String>: From<&'a mut Zval> {
        let zv_ptr = unsafe { ffi::zend_hash_index_find(self as *const _ as *mut _, idx) };
        if zv_ptr.is_null() {
            return Err(format!("No value for given index of {}", idx))
        }
        let zv: &mut Zval = unsafe { mem::transmute(zv_ptr) };
        // maybe we have to clone the zval here if it's reused by the caller..
        From::from(zv)
    }
}

#[derive(Debug)]
#[repr(C)]
struct ZendBucket {
    val: Zval,
    h: zend_ulong,
    key: CZendString
}
