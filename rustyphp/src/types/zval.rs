use zend_mm::*;
use php_config::*;
use types::*;
use ::ffi;

use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;

/// zval.u1.v.type_flags
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

#[derive(Debug, Clone)]
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
    write_property: extern fn(obj: *mut Zval, member: *mut Zval, val: *mut Zval, cache_slot: *mut *mut c_void) -> *mut Zval,
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
    refc: ZendRefcounted,
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
        // Zval for call handler (as obj ptr) (maybe cache it?)
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

    /// Assign a value to an object property
    pub fn write_property<T: AssignTo>(&mut self, name: &str, value: T) -> Option<String> {
        let mut member = ZvalGuard(Zval::new());
        name.assign_to(&mut member); //@alloc member
        let mut tmp = Zval::new();
        value.assign_to(&mut tmp);
        // Zval for call handler (as obj ptr) (maybe cache it?)
        let mut obj = Zval::new();
        self.assign_to(&mut obj);
        unsafe {
            if self.obj_handlers.is_null() {
                return Some(format!("write_property: object handler is null"))
            }
            let handler_write_property = (*self.obj_handlers).write_property;
            handler_write_property(&mut obj as *mut _, &mut member as &mut Zval, &mut tmp, ptr::null_mut());
        };
        None
    }
}

#[derive(Debug, Clone)]
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
        let ptr: *mut ZendRefcounted = unsafe { self.value.as_ptr_mut().data as *mut _ } ;
        unsafe { Refcounted::drop_ptr(ptr) };
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
    Array = 7,
    Object = 8
}
