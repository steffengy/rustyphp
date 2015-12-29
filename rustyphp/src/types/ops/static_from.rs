//! (static type) Conversion functions
//! Only allow static types for normal conversion (zval[T] -> T)
//! Basically a string containing "1" cannot be interpreted as integer that way

use std::mem;
use std::slice;
use std::str;
use types::*;
use zstr::{CZendString};

macro_rules! primitive_from_helper {
    (long, $zv:expr, $cast_as:ty) => (Ok($zv.value.data as $cast_as))
}
macro_rules! primitive_from {
    ($zval_from:ident => $($ty:ty),*) => {
        $(
            impl<'a> From<&'a mut Zval> for Result<$ty, String> {
            #[inline]
                fn from(zv: &mut Zval) -> Self {
                    if zv.type_() != ZvalType::Long as u32 {
                        return Err(format!("Zval Conversion: Got {} insteadof $zval_from", zv.type_()))
                    }
                    primitive_from_helper!($zval_from, zv, $ty)
                }
            }
        )*
    }
}
primitive_from!(long => i8, i16, i32, i64, u8, u16, u32, u64);

impl<'a> From<&'a mut Zval> for Result<&'a mut Zval, String> {
    #[inline]
    fn from(zv: &'a mut Zval) -> Result<&'a mut Zval, String> {
        Ok(zv)
    }
}
impl<'a> From<&'a mut Zval> for Result<&'a mut ZvalValueObject, String> {
    #[inline]
    fn from(zv: &'a mut Zval) -> Result<&'a mut ZvalValueObject, String> {
        if zv.type_() != ZvalType::Object as u32 {
            return Err(format!("Zval Conversion: Got {} insteadof object", zv.type_()))
        }
        Ok(unsafe {
            mem::transmute(zv.value.as_ptr_mut().data)
        })
    }
}

impl<'a> From<&'a mut Zval> for Result<String, String> {
    #[inline]
    fn from(zv: &'a mut Zval) -> Self {
        let tmp: Result<&'a str, String> = From::from(zv);
        tmp.map(|st| st.to_owned())
    }
}

impl<'a> From<&'a mut Zval> for Result<&'a str, String> {
    fn from(zv: & mut Zval) -> Self {
        if zv.type_() != ZvalType::String as u32 {
            return Err(format!("Zval Conversion: Got {} insteadof string", zv.type_()))
        }
        let slice: &[u8] = unsafe {
            let zs: &mut CZendString = mem::transmute(zv.value.as_ptr().data);
            slice::from_raw_parts(zs.value.as_ptr(), zs.header.len)
        };
        let str_ = match str::from_utf8(slice) {
            Ok(x) => x,
            Err(err) => return Err(format!("{}", err))
        };

        Ok(str_)
    }
}