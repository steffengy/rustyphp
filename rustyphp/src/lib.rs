extern crate libc;
use libc::{c_int, c_void, c_uchar, c_ushort, size_t};

use std::mem;
use std::ptr;

/// Wrappers for libc types
#[allow(non_camel_case_types)]
pub mod types {
    pub type c_void = ::libc::c_void;
    pub type c_int = ::libc::c_int;
    pub type c_uchar = ::libc::c_uchar;
}

#[derive(Debug)]
#[repr(C)]
pub struct ZvalValue {
    /// long value is not refcounted (not annoying to implement with unions)
    pub long: i64
}

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

pub struct ZendBuildOptions {
    pub module_api: c_int,
    pub debug: u8,
    pub zts: u8,
    pub build_id: &'static str
}

#[inline]
pub unsafe fn make_module(funcs: Box<[ZendFunctionEntry]>, cfg: ZendBuildOptions) -> ZendModuleEntry {
    let module = ZendModuleEntry {
        size: mem::size_of::<ZendModuleEntry>() as u16,
        zend_api: cfg.module_api,
        zend_debug: cfg.debug,
        zts: cfg.zts,
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
        build_id: cfg.build_id.as_ptr()
    };
    return module;
}

#[macro_export]
macro_rules! php_ext {
    ( $($k:ident => $v:expr)* ) => {
        use ::rustyphp::{ZendFunctionEntry, ZendModuleEntry, ZendBuildOptions};

        static mut MODULE_PTR: Option<ZendModuleEntry> = None;

        #[no_mangle]
        pub unsafe extern fn get_module() -> *mut ::rustyphp::types::c_void {
            if MODULE_PTR.is_none() {
                let sett = ZendBuildOptions {
                    module_api: ZEND_MODULE_API_NO,
                    debug: ZEND_DEBUG,
                    zts: ZEND_ZTS,
                    build_id: ZEND_MODULE_BUILD_ID
                };
                let mut module = rustyphp::make_module(Box::new(get_php_funcs!()), sett);
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
