use rustyphp::*;

#[php_func]
fn rustyphp_func_ret_u32() -> u32 {
    42
}
php_test!(i32, code => "var_dump(rustyphp_func_ret_u32());", expect => "int(42)");

/// This causes a copy of the string literal into a zend string structure
#[php_func]
fn rustyphp_func_ret_str() -> &'static str {
    "test"
}
php_test!(str, code => "var_dump(rustyphp_func_ret_str());", expect => "string(4) \"test\"");

/// This causes a copy of the string into a zend string structure
/// That means the string is allocated on stack and then copied (rust stack and zend MM)
#[php_func]
fn rustyphp_func_ret_string() -> String {
    format!("hello {}", "world")
}
php_test!(string, code => "var_dump(rustyphp_func_ret_string());", expect => "string(11) \"hello world\"");

#[php_func]
fn rustyphp_func_ret_arr_int() -> Vec<i32> {
    let g = vec![4,2,0,0,1];
    g
}
php_test!(arr_int, code => "echo implode('', rustyphp_func_ret_arr_int());", expect => "42001");

#[php_func]
fn rustyphp_func_ret_arr_str() -> Vec<String> {
    let mut g = vec![];
    for _ in 0..10 {
        g.push("a".to_owned());
    }
    g
}
php_test!(arr_str, code => "echo implode('', rustyphp_func_ret_arr_str());", expect => "aaaaaaaaaa");