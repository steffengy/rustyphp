extern crate rustyphp_zend_alloc;

extern crate rustyphp_core;

pub use rustyphp_core::*;
use rustyphp_core::types::*;

use std::mem;
use std::ptr;
pub use std::result;

///
#[derive(Debug)]
#[repr(C)]
pub struct ExecuteData
{
    opline: *mut c_void,
    pub call: *mut c_void,
    return_value: *mut c_void,
    func: *mut c_void,
    pub this: Zval,
    called_scope: *mut c_void,
    pub prev_execute_data: *mut c_void
}

impl ExecuteData {
    /// Get the arg count stored in zval (doesnt check if it's actually used for arg_count)
    pub fn arg_count(&self) -> usize {
        self.this.u2 as usize
    }

    /// Fetch an PHP argument from current_execute_data (first arg is idx = 0)
    pub fn arg(&mut self, idx: usize) -> &Zval {
        unsafe {
            &*((self as *mut _ as *mut Zval).offset(php_config::ZEND_CALL_FRAME_SLOT as isize + idx as isize))
        }
    }
}

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

#[repr(C)]
pub struct ZendFunctionEntry
{
    pub name: *const c_uchar,
    pub handler: Option<extern fn (&mut ExecuteData, &mut Zval) -> ()>,
    pub arg_info: *mut c_void,
    pub num_args: u32,
    pub flags: u32
}

#[inline]
pub unsafe fn make_module(funcs: Option<Box<[ZendFunctionEntry]>>) -> ZendModuleEntry {
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
            Some(funcs) => Box::into_raw(funcs) as *mut _
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
        static mut MODULE_PTR: Option<$crate::ZendModuleEntry> = None;

        #[no_mangle]
        pub unsafe extern fn get_module() -> *mut ::rustyphp::types::c_void {
            if MODULE_PTR.is_none() {
                let mut module = rustyphp::make_module(get_php_funcs!());
                $(
                    module.$k = $v;
                )*
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

// Macros

#[macro_export]
macro_rules! throw_exception {
    ($error:expr) => ({
        let str_ = ::std::ffi::CString::new($error).unwrap(); //TODO: replace by match, if this fails we have a problem (endless loop)
        unsafe { $crate::external::zend_throw_exception(::std::ptr::null_mut(), str_.as_ptr() as *mut _, 0) }
    })
}


// Exception handling wrappers
#[macro_export]
macro_rules! zend_try {
    ($expr:expr) => (
        match $expr {
            $crate::result::Result::Ok(x) => x,
            $crate::result::Result::Err(err) => {
                throw_exception!(err); 
                return
            }
        }
    )
}


#[macro_export]
macro_rules! zend_try_option {
    ($expr:expr) => (
        match $expr {
            None => {},
            Some(err) => {
                throw_exception!(err); 
                return
            }
        }
    )
}
