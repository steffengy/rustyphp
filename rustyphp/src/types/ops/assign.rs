//! Zval Assign: Allows to assign values to zvals using simply 5.assign_to(zv)
//! value -> zval

use std::mem;
use std::ptr;
use php_config::*;
use types::*;
use ffi;
use zstr::{CZendString, CZendStringHeader};

macro_rules! primitive_assign_help {
    ($target:expr, long, $_self:expr, $value_ty:ty) => ($target.value.data = *$_self as $value_ty);
    ($target:expr, $trans_func:ident, $_self:expr, $value_ty:ty) => (unsafe { $target.value.$trans_func() }.data = *$_self as $value_ty);
}
macro_rules! primitive_assign {
    // default type long
    ( $( $($from_ty:ty),* => $target_zvt:expr, $value_ty:ty ),* ) => {
        primitive_assign!($( $($from_ty),* => $target_zvt, $value_ty : long),*);
    };
    // root macro
    ( $( $($from_ty:ty),* => $target_zvt:expr, $value_ty:ty : $conv_ty:ident ),* ) => {
        $(
            $(
                impl AssignTo for $from_ty {
                    #[inline]
                    fn assign_to(&self, target: &mut Zval) -> Option<String> {
                        target.set_type($target_zvt);
                        primitive_assign_help!(target, $conv_ty, self, $value_ty);
                        None
                    }
                }
            )*
        )*
    }
}
pub trait AssignTo  {
    fn assign_to(&self, target: &mut Zval) -> Option<String>;
}

impl AssignTo for bool {
    #[inline]
    fn assign_to(&self, target: &mut Zval) -> Option<String> {
        target.set_type(match *self {
            true => ZvalType::True,
            false => ZvalType::False
        });
        None
    }
}

primitive_assign!(i8, i16, i32, i64, u8, u16, u32, u64 => ZvalType::Long, zend_long);
primitive_assign!(f64, f32 => ZvalType::Double, zend_double : as_double_mut);

impl AssignTo for ZvalValueObject {
    #[inline]
    fn assign_to(&self, target: &mut Zval) -> Option<String> {
        target.set_type(ZvalType::Object);
        let ptr = self as *const _ as *mut _;
        unsafe { target.value.as_ptr_mut().data = ptr };
        None
    }
}

impl<T: AssignTo> AssignTo for Option<T> {
    #[inline]
    fn assign_to(&self, target: &mut Zval) -> Option<String> {
        match *self {
            None => target.set_type(ZvalType::Null),
            Some(ref val) => { val.assign_to(target); }
        };
        None
    }
}

impl AssignTo for String {
    #[inline]
    fn assign_to(&self, target: &mut Zval) -> Option<String> {
        (self as &str).assign_to(target)
    }
}

impl<'a> AssignTo for &'a str {
    fn assign_to(&self, target: &mut Zval) -> Option<String> {
        let pzv: &mut ZvalValuePtr = unsafe { mem::transmute(&mut target.value) };

        // We need to allocate the string in the struct so we'll need a bit of dirty work..
        // since in PHP the zend_string struct contains an value array of size 1
        // [which is a hack to make addressing easier]
        let ptr = unsafe {
            zend_emalloc!(mem::size_of::<CZendString>() + self.len())
        };

        let header: &mut CZendString = unsafe { mem::transmute(ptr as *mut CZendStringHeader) };
        *header = CZendString {
            header: CZendStringHeader {
                refc: ZvalRefcounted { refcount: 1, type_info: ZvalType::String as u32 },
                h: 0,
                len: self.len()
            },
            value: [0u8]
        };
        unsafe {
            let dst_ptr = header.value.as_ptr() as *mut _;
            ptr::copy_nonoverlapping(self.as_bytes().as_ptr(), dst_ptr, self.len() as usize);
            *dst_ptr.offset(self.len() as isize) = 0;
        }

        pzv.data = ptr as *mut _;
        target.set_type(ZvalType::String);
        None
    }
}
