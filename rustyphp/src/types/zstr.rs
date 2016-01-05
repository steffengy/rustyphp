//! ZendString
use std::mem;
use std::ptr;
use super::*;
use php_config::*;
use zend_mm::*;
use ffi;

static IS_STR_PERSISTENT: u32 = (1<<0);

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
    pub fn new(len: usize, persistent: bool) -> Refcounted<CZendString> {
        let boxed = unsafe { zend_emalloc!(len + mem::size_of::<CZendString>(), persistent) };
        let ptr: &mut CZendString = unsafe { mem::transmute(boxed) };

        let mut flags = ZvalType::String as u32;
        if persistent {
            flags |= IS_STR_PERSISTENT << 8
        }
        *ptr = CZendString {
            header: CZendStringHeader {
                refc: ZendRefcounted {
                    refcount: 1,
                    type_info: flags
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
