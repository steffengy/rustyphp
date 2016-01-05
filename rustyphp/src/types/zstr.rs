//! ZendString
use std::mem;
use std::ptr;
use super::*;
use php_config::*;
use zend_mm::*;
use ffi;

#[derive(Debug)]
#[repr(C)]
pub struct CZendStringHeader {
    pub refc: ZendRefcounted,
    pub h: zend_ulong,
    pub len: size_t,
}

#[derive(Debug)]
#[repr(C)]
pub struct CZendString {
    pub header: CZendStringHeader,
    pub value: [c_uchar ;1]
}

impl CZendString {
    pub fn new(len: usize) -> Refcounted<CZendString> {
        let boxed = unsafe { zend_emalloc!(len + mem::size_of::<CZendString>()) };
        let ptr: &mut CZendString = unsafe { mem::transmute(boxed) };

        *ptr = CZendString {
            header: CZendStringHeader {
                refc: ZendRefcounted {
                    refcount: 1,
                    type_info: ZvalType::String as u32
                },
                h: 0,
                len: len,
            },
            value: [0u8]
        };
        Refcounted(ZendBox(ptr))
    }

    #[inline]
    pub fn set_value(&mut self, val: &[u8]) {
        unsafe {
            let dst_ptr = self.value.as_ptr() as *mut _;
            ptr::copy_nonoverlapping(val.as_ptr(), dst_ptr, val.len() as usize);
            *dst_ptr.offset(val.len() as isize) = 0;
        }
    }
}
