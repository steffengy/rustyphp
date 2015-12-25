#![feature(plugin, custom_attribute)]
#![plugin(rustyphp_plugin)]
extern crate libc;
use libc::{c_ushort, size_t};

use std::mem;
use std::ptr;

pub mod types;
pub mod php_config;
use php_config::*;
use types::*;

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
pub struct Zval {
    pub value: ZvalValue,
    pub u1: u32,
}

/// Dummy struct for now, that we do not need to pass raw pointers
pub struct ExecuteData;

#[derive(Debug)]
#[repr(C)]
pub struct ZendModuleEntry
{
    size: c_ushort,
    zend_api: c_int,
    zend_debug: c_uchar,
    zts: c_uchar,
    ini_entry: *mut c_void,
    deps: *mut c_void,
    pub name: *const libc::c_uchar,
    functions: *mut ZendFunctionEntry,
    // INIT_FUNC_ARGS int type, int module_number
    pub module_startup_func: Option<extern fn(c_int, c_int) -> c_int>,
    request_startup_func: Option<extern fn(c_int, c_int) -> c_int>,
    // SHUTDOWN_FUNC_ARGS int type, int module_number
    module_shutdown_func: Option<extern fn(c_int, c_int) -> c_int>,
    request_shutdown_func:  Option<extern fn(c_int, c_int) -> c_int>,
    info_func: Option<extern fn(*mut ZendModuleEntry) -> *mut c_void>,
    pub version: *const libc::c_uchar,
    globals_size: size_t,
    globals_ptr: *mut c_void,
    globals_ctor: Option<extern fn(*mut c_void) -> *mut c_void>,
    globals_dtor: Option<extern fn(*mut c_void) -> *mut c_void>,
    post_deactivate_func: Option<extern fn() -> c_int>,
    module_started: c_int,
    ztype: c_uchar,
    handle: *mut c_void,
    module_number: c_int,
    build_id: *const libc::c_uchar
}
unsafe impl Sync for ZendModuleEntry { }

#[repr(C)]
pub struct ZendFunctionEntry
{
    pub name: *const libc::c_uchar,
    pub handler: Option<extern fn (&mut ExecuteData, &mut Zval) -> ()>,
    pub arg_info: *mut c_void,
    pub num_args: u32,
    pub flags: u32
}

#[inline]
pub unsafe fn make_module(funcs: Box<[ZendFunctionEntry]>) -> ZendModuleEntry {
    let module = ZendModuleEntry {
        size: mem::size_of::<ZendModuleEntry>() as u16,
        zend_api: ZEND_MODULE_API_NO,
        zend_debug: ZEND_DEBUG,
        zts: ZEND_ZTS,
        ini_entry: ptr::null_mut(),
        deps: ptr::null_mut(),
        name: ptr::null_mut(),
        functions: Box::into_raw(funcs) as *mut _,
        module_startup_func: None,
        request_startup_func: None,
        module_shutdown_func: None,
        request_shutdown_func: None,
        info_func: None,
        version: ptr::null_mut(),
        globals_size: 0,
        globals_ptr: ptr::null_mut(),
        globals_ctor: None,
        globals_dtor: None,
        post_deactivate_func: None,
        module_started: 0,
        ztype: 0,
        handle: ptr::null_mut(),
        module_number: 0,
        build_id: ZEND_MODULE_BUILD_ID.as_ptr()
    };
    return module;
}

#[macro_export]
macro_rules! php_ext {
    ( $($k:ident => $v:expr)* ) => {
        use ::rustyphp::{ZendFunctionEntry, ZendModuleEntry};

        static mut MODULE_PTR: Option<ZendModuleEntry> = None;

        #[no_mangle]
        pub unsafe extern fn get_module() -> *mut ::rustyphp::types::c_void {
            if MODULE_PTR.is_none() {
                let mut module = rustyphp::make_module(Box::new(get_php_funcs!()));
                $(
                    module.$k = $v;
                )*
                assert!(module.name != ::std::ptr::null_mut(), "Extension name cannot be null");
                assert!(module.version != ::std::ptr::null_mut(), "Extension version cannot be null");
                MODULE_PTR = Some(module)
            }
            match MODULE_PTR {
                None => panic!("Could not get module ptr"),
                Some(ref mut val) => val as *mut ZendModuleEntry as *mut _
            }
        }
    }
}
