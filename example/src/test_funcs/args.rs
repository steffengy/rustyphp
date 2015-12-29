use rustyphp::ZvalValueObject;

#[php_func]
fn rustyphp_func_arg_i32(p1: i32) {
    println!("RUST_PRINTLN({})", p1)
}
php_test!(i32, code => "rustyphp_func_arg_i32(42);", expect => "RUST_PRINTLN(42)");

#[php_func]
fn rustyphp_func_arg_obj(p1: &mut ZvalValueObject) {
    match p1.read_property::<u32>("prop") {
        Err(err) => println!("RUST_ERR({:?})", err),
        Ok(x) => println!("RUST_PRINTLN({})", x)
    }
}
php_test!(obj, code => "$g=new stdClass();$g->prop=1;rustyphp_func_arg_obj($g);", expect => "RUST_PRINTLN(1)");
//TODO: Mark function arguments as required, overwrite via Optional<Arg> which is a typedef
//to Option<Arg> but can be detected in the AST :)
php_test!(
    missing_arg_obj, status_success => false,
    code => "rustyphp_func_arg_obj();",
    expect => "ERR: stdout: Fatal error: Uncaught Exception: rustyphp_func_arg_obj: expected 1 arguments got 0",
    check_func => |expect: &str, stdout: &str, _| { stdout.starts_with(expect); }
);
