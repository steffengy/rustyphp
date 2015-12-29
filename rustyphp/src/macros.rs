macro_rules! zend_emalloc {
    ($size:expr) => (ffi::_emalloc($size, file!().as_ptr(), line!(), file!().as_ptr(), line!()))
}

macro_rules! convert_zval {
    ($conversion_func:ident, $zv:expr) => {
        unsafe { ffi::$conversion_func($zv as *const _ as *mut _); }
    }
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

#[macro_export]
macro_rules! throw_exception {
    ($error:expr) => ({
        let str_ = ::std::ffi::CString::new($error).unwrap(); //TODO: replace by match, if this fails we have a problem (endless loop)
        unsafe { $crate::ffi::zend_throw_exception(::std::ptr::null_mut(), str_.as_ptr() as *mut _, 0) }
    })
}

#[macro_export]
macro_rules! verify_arg_count {
    ($fn_:expr, $ex:expr, $req_args:expr) => {
        if $ex.arg_count() < $req_args {
            throw_exception!(format!("{}: expected {} arguments got {}", $fn_, $req_args, $ex.arg_count()));
            return;
        }
    }
}

fn halo(e: &::types::execute_data::ExecuteData) {
    verify_arg_count!("abc", e, 5);
}