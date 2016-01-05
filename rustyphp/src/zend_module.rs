//! Definitions which are relevant for meta definitions (function entries, module entries, class entries, ...)

use std::mem;
use std::ptr;
use super::*;

use ::types::execute_data::ExecuteData;
use ::types::zstr::CZendString;

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
    pub name: *const c_uchar,
    functions: *mut ZendFunctionEntry,
    // INIT_FUNC_ARGS int type, int module_number
    pub module_startup_func: Option<extern fn(c_int, c_int) -> c_int>,
    request_startup_func: Option<extern fn(c_int, c_int) -> c_int>,
    // SHUTDOWN_FUNC_ARGS int type, int module_number
    module_shutdown_func: Option<extern fn(c_int, c_int) -> c_int>,
    request_shutdown_func:  Option<extern fn(c_int, c_int) -> c_int>,
    info_func: Option<extern fn(*mut ZendModuleEntry) -> *mut c_void>,
    pub version: *const c_uchar,
    globals_size: size_t,
    globals_ptr: *mut c_void,
    globals_ctor: Option<extern fn(*mut c_void) -> *mut c_void>,
    globals_dtor: Option<extern fn(*mut c_void) -> *mut c_void>,
    post_deactivate_func: Option<extern fn() -> c_int>,
    module_started: c_int,
    ztype: c_uchar,
    handle: *mut c_void,
    module_number: c_int,
    build_id: *const c_uchar
}
unsafe impl Sync for ZendModuleEntry { }

/// _zend_internal_arg_info
/// For [arg_info] idx=0 this whole struct actually _zend_internal_function_info
/// providing general information about the func (a kind of header element)
#[repr(C)]
pub struct ZendInternalArgInfo {
    /// Either the argument name (idx>0)
    /// or the required_args count (i32 for idx=0, where this whole struct actually is _zend_internal_function_info)
    pub arg_name: *const c_uchar,
    pub cls_name: *const c_uchar,
    pub type_hint: c_uchar,
    pub pass_by_ref: c_uchar,
    pub allow_null: bool,
    pub is_variadic: bool
}

#[repr(C)]
pub struct ZendFunctionEntry
{
    pub name: *const c_uchar,
    pub handler: Option<extern fn (&mut ExecuteData, &mut Zval) -> ()>,
    pub arg_info: *mut c_void,
    pub num_args: u32,
    pub flags: u32
}

#[repr(C)]
pub struct ZendClassEntry
{
    ty: c_uchar,
    name: *mut CZendString,
    parent: *const ZendClassEntry,
    refcount: c_int,
    ce_flags: u32,
    default_prop_count: c_int,
    default_static_member_count: c_int,
    default_prop_table: *mut Zval,
    default_static_member_table: *mut Zval,
    static_members_table: *mut Zval,
    functions: ZendArray,
    properties: ZendArray,
    constants: ZendArray,
    //to be completed when needed...
}

#[inline]
pub unsafe fn make_module(funcs: Option<*mut ZendFunctionEntry>) -> ZendModuleEntry {
    let module = ZendModuleEntry {
        size: mem::size_of::<ZendModuleEntry>() as u16,
        zend_api: ZEND_MODULE_API_NO,
        zend_debug: ZEND_DEBUG,
        zts: ZEND_ZTS,
        ini_entry: ptr::null_mut(),
        deps: ptr::null_mut(),
        name: ptr::null_mut(),
        functions: match funcs {
            None => ptr::null_mut(),
            Some(funcs) => funcs
        },
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
        static mut MODULE_PTR: Option<::rustyphp::ZendModuleEntry> = None;
        static mut WRAPPED_STARTUP_FUNC: Option<extern fn(c_int, c_int) -> c_int> = None;
        get_php_funcs!();

        extern fn startup_wrapper(ty: c_int, module_number: c_int) -> c_int {
            unsafe {
                // register classes

                match WRAPPED_STARTUP_FUNC {
                    Some(func) => func(ty, module_number),
                    _ => 0
                }
            };
            1
        }

        #[no_mangle]
        pub unsafe extern fn get_module() -> *mut ::rustyphp::types::c_void {
            if MODULE_PTR.is_none() {
                let mut module = rustyphp::make_module(Some(FUNC_PTR.as_mut_ptr()));
                $(
                    module.$k = $v;
                )*
                // wrap the module startup func since we need it
                WRAPPED_STARTUP_FUNC = module.module_startup_func;
                module.module_startup_func = Some(startup_wrapper);

                assert!(module.name != ::std::ptr::null_mut(), "Extension name cannot be null");
                assert!(module.version != ::std::ptr::null_mut(), "Extension version cannot be null");
                MODULE_PTR = Some(module)
            }
            match MODULE_PTR {
                None => panic!("Could not get module ptr"),
                Some(ref mut val) => val as *mut $crate::ZendModuleEntry as *mut _
            }
        }
    }
}
