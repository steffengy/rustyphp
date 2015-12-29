
#[php_func]
fn rustyphp_func_arg_i32(p1: i32) {
    println!("RUST_PRINTLN({})", p1)
}
php_test!(i32, "rustyphp_func_arg_i32(42);" => "RUST_PRINTLN(42)");
