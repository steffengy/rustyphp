//! ZendString
use super::*;
use php_config::*;
use zend_mm::*;

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
        let str_ = CZendString {
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
        Refcounted::new(str_)
    }
}
