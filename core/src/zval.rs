use php_config::*;
use types::*;
use alloc::heap;
use std::ops::Deref;
use std::mem;
use std::ptr;

/* zval.u1.v.type_flags */
static IS_TYPE_CONSTANT: u32 = (1<<0);
static IS_TYPE_IMMUTABLE: u32 = (1<<1);
static IS_TYPE_REFCOUNTED: u32 = (1<<2);
static IS_TYPE_COLLECTABLE: u32 = (1<<3);
static IS_TYPE_COPYABLE: u32 = (1<<4);
static Z_TYPE_FLAGS_SHIFT: u32 = 8;

macro_rules! union {
    ($base:ident, $variant:ident, $variant_mut:ident, $otherty:ty) => {
        impl $base {
            #[inline]
            pub unsafe fn $variant(&self) -> &$otherty {
                ::std::mem::transmute(self)
            }
            
            #[inline]
            pub unsafe fn $variant_mut(&mut self) -> &mut $otherty {
                ::std::mem::transmute(self)
            }
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValue {
    /// long value is not refcounted (not annoying to implement with unions)
    pub data: zend_long
}

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValueDouble {
    /// float/double value
    pub data: zend_double
}
union!(ZvalValue, as_double, as_double_mut, ZvalValueDouble);

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValuePtr {
    pub data: *mut c_void
}

#[derive(Debug)]
#[repr(C)]
struct CZendStringHeader {
    refc: ZvalRefcounted,
    h: zend_ulong,
    len: size_t,
}

// (internal) zend_string
#[derive(Debug)]
#[repr(C)]
struct CZendString {
    header: CZendStringHeader,
    val: *mut c_uchar
}

#[derive(Debug)]
#[repr(C)]
struct ZvalRefcounted {
    refcount: u32,
    type_info: u32
}

#[derive(Debug)]
#[repr(C)]
pub struct Zval {
    pub value: ZvalValue,
    pub u1: u32,
    pub u2: u32
}

impl Zval {
    #[inline]
    pub fn type_(&self) -> u32 {
        self.u1 & 0xFF
    }

    #[inline]
    pub fn set_type(&mut self, type_: ZvalType) {
        self.u1 = match type_ {
            ZvalType::String => ZvalType::String as u32 | ((IS_TYPE_REFCOUNTED | IS_TYPE_COPYABLE) << Z_TYPE_FLAGS_SHIFT),
            // primitives
            _ => type_ as u32
        };
        self.u2 = 0;
    }
}

#[repr(u32)]
pub enum ZvalType {
    Null = 1,
    False = 2,
    True = 3,
    Long = 4,
    Double = 5,
    String = 6
}

// Assigning functions
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
    fn assign_to(&self, target: &mut Zval) -> Option<String> {
        let pzv: &mut ZvalValuePtr = unsafe { mem::transmute(&mut target.value) };

        // We need to allocate the string in the struct so we'll need a bit of dirty work..
        // since in PHP the zend_string struct contains an value array of size 1 
        // [which is a hack to make addressing easier]
        let ptr = unsafe { 
            heap::allocate(mem::size_of::<CZendStringHeader>() + self.len() + 1, mem::align_of::<CZendStringHeader>()) 
        };

        let header: &mut CZendStringHeader = unsafe { mem::transmute(ptr as *mut CZendStringHeader) };
        *header = CZendStringHeader {
            refc: ZvalRefcounted { refcount: 1, type_info: ZvalType::String as u32 },
            h: 0,
            len: self.len()
        };
        unsafe { 
            let dst_ptr = ptr.offset(mem::size_of::<CZendStringHeader>() as isize) as *mut _;
            ptr::copy_nonoverlapping(self.as_bytes().as_ptr(), dst_ptr, self.len() as usize);
            *dst_ptr.offset(self.len() as isize) = 0;
        }

        pzv.data = ptr as *mut _;
        target.set_type(ZvalType::String);
        None
    }
}

// (static type) Conversion functions
// Only allow static types for normal conversion
// Basically a string containing "1" cannot be interpreted as integer that way
impl<'a> From<&'a Zval> for Result<u16, String> {
    fn from(zv: &Zval) -> Self {
        if zv.type_() != ZvalType::Long as u32 {
            return Err(format!("Zval Conversion: Cannot convert {} to long", zv.type_()))
        }
        Ok(zv.value.data as u16)
    }
}

/// (dynamic type) Conversion functions
/// Convert the data type if it is not matching
pub struct ConvertZvalAs<T>(T);

impl<T> Deref for ConvertZvalAs<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl <'a> From<&'a Zval> for Result<ConvertZvalAs<u16>, String> {
    fn from(zv: &Zval) -> Result<ConvertZvalAs<u16>, String> {
        convert_zval!(convert_to_long, zv);
        Ok(ConvertZvalAs(try!(From::from(zv))))
    } 
}
