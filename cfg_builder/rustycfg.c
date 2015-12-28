/*
  +----------------------------------------------------------------------+
  | PHP Version 7                                                        |
  +----------------------------------------------------------------------+
  | Copyright (c) 1997-2015 The PHP Group                                |
  +----------------------------------------------------------------------+
  | This source file is subject to version 3.01 of the PHP license,      |
  | that is bundled with this package in the file LICENSE, and is        |
  | available through the world-wide-web at the following url:           |
  | http://www.php.net/license/3_01.txt                                  |
  | If you did not receive a copy of the PHP license and are unable to   |
  | obtain it through the world-wide-web, please send a note to          |
  | license@php.net so we can mail you a copy immediately.               |
  +----------------------------------------------------------------------+
  | Author:                                                              |
  +----------------------------------------------------------------------+
*/

/* $Id$ */

#ifdef HAVE_CONFIG_H
#include "config.h"
#endif

#include "php.h"
#include "php_ini.h"
#include "ext/standard/info.h"
#include "php_rustycfg.h"

PHP_FUNCTION(dump_rusty_config)
{
	zend_string *strg;

	strg = strpprintf(0,
        "use types::*;\n"
        "pub static ZEND_MODULE_API_NO: c_int = %d;\n"
        "pub static ZEND_MODULE_BUILD_ID: &'static str = \"%s\\0\";\n"
        "pub static ZEND_ZTS: c_uchar = %d;\n"
        "pub static ZEND_DEBUG: c_uchar = %d;\n"
        "pub static ZEND_CALL_FRAME_SLOT: c_int = %d;\n"
        "/// zend_long\n#[allow(non_camel_case_types)]\npub type zend_long = i%d;\n"
        "/// zend_ulong\n#[allow(non_camel_case_types)]\npub type zend_ulong = u%d;\n"
        "/// zend_double\n#[allow(non_camel_case_types)]\npub type zend_double = f%d;\n\n"
    ,
        ZEND_MODULE_API_NO,
        ZEND_MODULE_BUILD_ID,
        #ifdef ZTS
        1,
        #else
        0,
        #endif
        ZEND_DEBUG,
        ZEND_CALL_FRAME_SLOT,
        sizeof(zend_long) * 8,
        sizeof(zend_ulong) * 8,
        sizeof(double) * 8
    );

	RETURN_STR(strg);
}

const zend_function_entry rustycfg_functions[] = {
	PHP_FE(dump_rusty_config,	NULL)
	PHP_FE_END
};

zend_module_entry rustycfg_module_entry = {
	STANDARD_MODULE_HEADER,
	"rustycfg",
	rustycfg_functions,
	NULL,
	NULL,
	NULL,
	NULL,
	NULL,
	PHP_RUSTYCFG_VERSION,
	STANDARD_MODULE_PROPERTIES
};
/* }}} */

#ifdef COMPILE_DL_RUSTYCFG
#ifdef ZTS
ZEND_TSRMLS_CACHE_DEFINE();
#endif
ZEND_GET_MODULE(rustycfg)
#endif
