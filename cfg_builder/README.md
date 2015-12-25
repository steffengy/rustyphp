Config Builder (Rustycfg)
=========================

The config builder is an essential component to generate type definitions
for rust to allow building compatible PHP extensions

Building Instructions
============
For *Windows* replace in the commands `rustycfg.so` with `php_rustycfg.dll`
and use phpize from a dev-pack.

```sh
$ phpize && ./configure --enable-rustycfg
$ make
$ php -dextension=rustycfg.so make.php
```

Then copy the php_config.rs to your rust-php extension
