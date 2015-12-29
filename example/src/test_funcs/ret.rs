use rustyphp::*;

#[php_func]
fn rustyphp_func_ret_u32() -> u32 {
    42
}
php_test!(i32, "var_dump(rustyphp_func_ret_u32(42));" => "int(42)");
