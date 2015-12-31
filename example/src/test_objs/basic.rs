
//Implemenation plan for Objects:
//TODO: allocate zend_class_entry
//TODO: write_property/read_property handlers
//TODO: zval instance caching (cache a constructed instance the first time rust-->zend boundaries are crossed/ when dynamic properties are used)

/*/// The difference between `rust_property` and `property` is that property is accessible from PHP
#[php_cls]
RustyPhpBasicObj {
    rust_property: i32,
    pub property: i32,
}*/

