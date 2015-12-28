use php_config::*;
use types::*;
use std::slice;
use std::str;
use std::ops::{Deref, DerefMut};
use std::mem;
use std::ptr;
use super::external;

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
struct ZvalRefcounted {
    refcount: u32,
    type_info: u32
}

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValueDouble {
    /// float/double value
    pub data: zend_double
}
union!(ZvalValue, as_double, as_double_mut, ZvalValueDouble);
union!(ZvalValue, as_ptr, as_ptr_mut, ZvalValuePtr);

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValuePtr {
    pub data: *mut c_void
}

#[derive(Debug)]
#[repr(C)]
#[allow(dead_code)]
struct ZendObjectHandlers {
    /* offset of real object header (usually zero) */
    offset: c_int,
    /* general object functions */
    free_obj: *mut c_void,
    dtor_obj: *mut c_void,
    clone_obj: *mut c_void,
    /* individual object functions */
    read_property: extern fn (obj: *mut Zval, member: *mut Zval, ty: c_int, cache_slot: *mut *mut c_void, rv: *mut Zval) -> *mut Zval,
    write_property: *mut c_void,
    read_dimension: *mut c_void,
    write_dimension: *mut c_void,
    get_property_ptr_ptr: *mut c_void,
    get: *mut c_void,
    set: *mut c_void,
    has_property: *mut c_void,
    unset_property: *mut c_void,
    has_dimension: *mut c_void,
    unset_dimension: *mut c_void,
    get_properties: *mut c_void,
    get_method: *mut c_void,
    call_method: *mut c_void,
    get_constructor: *mut c_void,
    get_class_name: *mut c_void,
    compare_objects: *mut c_void,
    cast_object: *mut c_void,
    count_elements: *mut c_void,
    get_debug_info: *mut c_void,
    get_closure: *mut c_void,
    get_gc: *mut c_void,
    do_operation: *mut c_void,
    compare: *mut c_void
}

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValueObject {
    refc: ZvalRefcounted,
    handle: u32,
    ce: *mut c_void,
    obj_handlers: *mut ZendObjectHandlers,
    properties: *mut c_void,
    prop_table: [Zval; 1]
}

impl<'a> ZvalValueObject {
    /// Read a property from the object
    pub fn read_property<T>(&mut self, name: &str) -> Result<T, String> where Result<T, String>: From<&'a mut Zval> {
        let mut member = ZvalGuard(Zval::new());
        name.assign_to(&mut member); //@alloc member
        // Zval for call handler (as obj ptr)
        let mut obj = Zval::new();
        self.assign_to(&mut obj);
        // Temporary zval which might be used by zend read handler (to reduce allocations)
        let mut zv = Zval::new(); // shouldnt alloc
        // 0 = BP_VAR_R
        let value: &mut Zval;
        unsafe {
            if self.obj_handlers.is_null() {
                return Err(format!("read_property: object handler is null"))
            }
            let handler_read_property = (*self.obj_handlers).read_property;
            // Using cache_slot and the underlying caching does virtually not bring a huge speed advantage
            value = mem::transmute(handler_read_property(&mut obj as *mut _, &mut member as &mut Zval, 0, ptr::null_mut(), &mut zv as *mut _));
        };

        // Err("test".to_owned())
        From::from(value)
    }
}

#[derive(Debug)]
#[repr(C)]
struct CZendStringHeader {
    refc: ZvalRefcounted,
    h: zend_ulong,
    len: size_t,
}

#[derive(Debug)]
#[repr(C)]
struct CZendString {
    header: CZendStringHeader,
    value: [c_uchar ;1]
}

#[derive(Debug)]
#[repr(C)]
pub struct Zval {
    pub value: ZvalValue,
    pub u1: u32,
    pub u2: u32
}

/// Allocation drop guard
#[derive(Debug)]
pub struct ZvalGuard(Zval);

/// Ensures not to leak memory
impl Drop for ZvalGuard {
    fn drop(&mut self) {
        // Check if the current type is refcounted
        if (self.type_flags() & IS_TYPE_REFCOUNTED) != IS_TYPE_REFCOUNTED {
            return;
        }
        // Make sure we can access the refcounted structure
        let rc: &mut ZvalRefcounted = unsafe { mem::transmute(self.value.as_ptr_mut().data) };
        // If it's only referenced in this scope, we can kill it
        if rc.refcount <= 1 {
            unsafe { external::_zval_dtor_func(mem::transmute(rc), file!().as_ptr() as *mut _, line!()); }
        } else {
            rc.refcount -= 1;
        }
    }
}

impl Deref for ZvalGuard {
    type Target = Zval;

    fn deref(&self) -> &Zval {
        &self.0
    }
}

impl DerefMut for ZvalGuard {
    fn deref_mut<'a>(&'a mut self) -> &'a mut Zval {
        &mut self.0
    }
}

impl Zval {
    pub fn new() -> Zval {
        Zval {
            value: ZvalValue { data: 0 },
            u1: 0,
            u2: 0
        }
    }

    #[inline]
    pub fn type_(&self) -> u32 {
        self.u1 & 0xFF
    }

    #[inline]
    pub fn type_flags(&self) -> u32 {
        (self.u1 >> 8) & 0xFF
    }

    #[inline]
    pub fn set_type(&mut self, type_: ZvalType) {
        self.u1 = match type_ {
            ZvalType::String => ZvalType::String as u32 | ((IS_TYPE_REFCOUNTED | IS_TYPE_COPYABLE) << Z_TYPE_FLAGS_SHIFT),
            ZvalType::Object => ZvalType::Object as u32 | ((IS_TYPE_REFCOUNTED | IS_TYPE_COLLECTABLE) << Z_TYPE_FLAGS_SHIFT),
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
    String = 6,
    Object = 8
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

// (static type) Conversion functions
// Only allow static types for normal conversion
// Basically a string containing "1" cannot be interpreted as integer that way
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

impl<'a> From<&'a mut Zval> for Result<u16, String> {
    #[inline]
    fn from(zv: &mut Zval) -> Self {
        if zv.type_() != ZvalType::Long as u32 {
            return Err(format!("Zval Conversion: Got {} insteadof long", zv.type_()))
        }
        Ok(zv.value.data as u16)
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

/// (dynamic type) Conversion functions
/// Convert the data type if it is not matching
pub struct ConvertZvalAs<T>(T);

impl<T> Deref for ConvertZvalAs<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl <'a> From<&'a mut Zval> for Result<ConvertZvalAs<u16>, String> {
    fn from(zv: &mut Zval) -> Result<ConvertZvalAs<u16>, String> {
        convert_zval!(convert_to_long, zv);
        Ok(ConvertZvalAs(try!(From::from(zv))))
    }
}
