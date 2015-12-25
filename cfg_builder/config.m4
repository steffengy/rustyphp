[  --enable-rustyfg           Enable test support])

if test "$PHP_RUSTYCFG" != "no"; then
  PHP_NEW_EXTENSION(rustycfg, rustycfg.c, $ext_shared,, -DZEND_ENABLE_STATIC_TSRMLS_CACHE=1)
fi
