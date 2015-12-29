/// (dynamic type) Conversion functions (zval[any] -> T)
/// Convert the data type if it is not matching

use std::ops::{Deref};
use php_config::*;
use types::*;
use ffi;

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
