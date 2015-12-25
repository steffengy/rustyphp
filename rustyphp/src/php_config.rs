use types::*;
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

