use rustyphp::*;

#[php_func]
fn rustyphp_func_ret_u32() -> u32 {
    42
}
php_test!(i32, code => "var_dump(rustyphp_func_ret_u32(42));", expect => "int(42)");
