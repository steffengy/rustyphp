//! ZendString
use super::*;
use php_config::*;

#[derive(Debug)]
#[repr(C)]
pub struct CZendStringHeader {
    pub refc: ZvalRefcounted,
    pub h: zend_ulong,
    pub len: size_t,
}

#[derive(Debug)]
#[repr(C)]
pub struct CZendString {
    pub header: CZendStringHeader,
    pub value: [c_uchar ;1]
}
