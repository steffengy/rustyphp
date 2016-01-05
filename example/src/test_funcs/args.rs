use rustyphp::{ZendArray, ZvalValueObject};

#[php_func]
fn rustyphp_func_arg_i32(p1: i32) {
    println!("RUST_PRINTLN({})", p1)
}
php_test!(i32, code => "rustyphp_func_arg_i32(42);", expect => "RUST_PRINTLN(42)");

#[php_func]
fn rustyphp_func_arg_arr(p1: &mut ZendArray) {
    let val1: &str = p1.get(42).unwrap();
    let val2: &str = p1.get(666).unwrap();
    println!("RS_ARR[42]={}\nRS_ARR[666]={}", val1, val2);
    match p1.get::<i32>(0) {
        Err(_) => println!("RUST_OK"),
        _ => println!("RUST_FAIL")
    }
}

php_test!(arr, code => "rustyphp_func_arg_arr(array(42 => \"hell yeah\", 666 => \"devil\"));", expect => "RS_ARR[42]=hell yeah\nRS_ARR[666]=devil\nRUST_OK");

#[php_func]
fn rustyphp_func_arg_obj(p1: &mut ZvalValueObject) {
    match p1.read_property::<u32>("prop") {
        Err(err) => println!("RUST_ERR({:?})", err),
        Ok(x) => println!("RUST_PRINTLN({})", x)
    }
}
php_test!(obj, code => "$g=new stdClass();$g->prop=1;rustyphp_func_arg_obj($g);", expect => "RUST_PRINTLN(1)");

#[php_func]
fn rustyphp_func_arg_obj_write(p1: &mut ZvalValueObject) {
    p1.write_property("prop", "yep");
}
php_test!(obj_write, code => "$g=new stdClass();$g->prop=\"a\";rustyphp_func_arg_obj_write($g);var_dump($g->prop);", expect => "string(3) \"yep\"");

/// Verify that property reading does free the zend_string for the property name
#[php_func]
fn rustyphp_func_arg_obj_memsafety(p1: &mut ZvalValueObject) {
    match p1.read_property::<u32>("prop") {
        Err(err) => println!("RUST_ERR({:?})", err),
        Ok(x) => assert_eq!(x, 2)
    }
}
php_test!(obj_memsafety_read_prop,
    code => "$g=new stdClass(); $g->prop = 2; $before = memory_get_usage(); for ($c = 0; $c < 100; ++$c) { rustyphp_func_arg_obj_memsafety($g); } \
    $after=memory_get_usage(); $status = ($before==$after?'Y':'N'); echo $status.'('.$before.','.$after.')';",
    expect => "Y(",
    check_func => |expect: &str, stdout: &str, _| { stdout.starts_with(expect); }
);

//TODO: Mark function arguments as required, overwrite via Optional<Arg> which is a typedef
//to Option<Arg> but can be detected in the AST :)
php_test!(
    missing_arg_obj, status_success => false,
    code => "rustyphp_func_arg_obj();",
    expect => "ERR: stdout: Fatal error: Uncaught Exception: rustyphp_func_arg_obj: expected 1 arguments got 0",
    check_func => |expect: &str, stdout: &str, _| { stdout.starts_with(expect); }
);
