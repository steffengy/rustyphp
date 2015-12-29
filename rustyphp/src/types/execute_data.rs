use std::mem;

use super::*;
use ::php_config;

//
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
    #[inline]
    pub fn arg_count(&self) -> usize {
        self.this.u2 as usize
    }

    /// Fetch an PHP argument from current_execute_data (first arg is idx = 0)
    #[inline]
    pub fn arg(&mut self, idx: usize) -> &mut Zval {
        unsafe {
            mem::transmute((self as *mut _ as *mut Zval).offset(php_config::ZEND_CALL_FRAME_SLOT as isize + idx as isize))
        }
    }
}
