#![feature(plugin_registrar, rustc_private)]

extern crate aster;
extern crate syntax;
extern crate rustc_plugin;

use std::cell::RefCell;

use aster::AstBuilder;
use rustc_plugin::Registry;
use syntax::ast::{Expr, Item_, MetaItem, FunctionRetTy, TokenTree};
use syntax::ext::base::{MacResult, MacEager, ExtCtxt};
use syntax::codemap::Span;
use syntax::ext::base::Annotatable;
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::parse::token::intern;

use syntax::ptr::P;

thread_local!(static REGISTERED_FUNCS: RefCell<Vec<RegisteredFunc>> = RefCell::new(vec![]));

enum VarType {
    Long
}

#[derive(Debug, Clone)]
struct RegisteredFunc {
    /// The internal name (mostly zif_ prefix for the C-definition)
    internal_name: String,
    /// The real name of the func
    real_name: String
}

#[plugin_registrar]
pub fn registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("php_func"), MultiDecorator(Box::new(expand_php_func)));
    reg.register_macro("get_php_funcs", get_php_funcs);
}

/// #[php_func] to declare exported php functions
fn expand_php_func(_: &mut ExtCtxt, _: Span, _: &MetaItem, anno: &Annotatable, push: &mut FnMut(Annotatable)) {
    let builder = AstBuilder::new();

    // Gather information about the original function
    // like return_value type
    let return_type;
    let orig_fn_item;

    match anno {
        &Annotatable::Item(ref item) => {
            orig_fn_item = item;
            match item.node {
                Item_::ItemFn(ref fn_decl, _, _, _, _, _) => {
                    match fn_decl.output {
                        FunctionRetTy::NoReturn(_) | FunctionRetTy::DefaultReturn(_) => return_type = None,
                        FunctionRetTy::Return(ref _type) => return_type = Some(_type)
                    }
                },
                _ => panic!("php_func: expected ItemFn, got {:?}", item.node)
            }
        },
        _ => panic!("php_func: Expected Item, got {:?}", anno)
    }

    // Generate wrapper function, which is callable from PHP userland
    let old_fn = orig_fn_item.ident.name;
    let new_fn = format!("zif_{}", old_fn);
    let field;

    match return_type {
        None => panic!("unsupported"),
        Some(ty) => {
            match &*syntax::print::pprust::ty_to_string(ty) {
                "i64" => field = VarType::Long,
                _ => panic!("php_func: Type not supported: {:?}", ty)
            }
        }
    }

    let value_ret = builder.expr().path().id("ret").build();
    let value_zv = builder.expr().path().id("zv").build();
    let field_type = builder.expr().field("u1").build(value_zv.clone());
    let value_zend_type;

    let value_field = match field {
        VarType::Long => {
            value_zend_type = builder.expr().u32(4);
            "long"
        }
    };
    let field_value = builder.expr().field(value_field).field("value").build(value_zv);

    let block = builder.block()
        .stmt().let_id("ret").call()
            .path().id(old_fn).build()
            .build()
        .stmt().semi().build_assign(field_value, value_ret)
        .stmt().semi().build_assign(field_type, value_zend_type)
        .build();

    let fn_ = builder.item()
        .attr().inline()
        .fn_(new_fn.clone())
            .arg("_").ty().ref_().mut_().ty().path()
                .global().ids(&["rustyphp", "ExecuteData"]).build() //execute_data as execute_data *
            .arg("zv").ty().ref_().mut_().ty().path()
                .global().ids(&["rustyphp", "Zval"]).build() //return_value as zval *
            .default_return()
            .abi(syntax::abi::Abi::C)
            .build(block);

    REGISTERED_FUNCS.with(|rf| {
        (*rf.borrow_mut()).push(RegisteredFunc {
            internal_name: new_fn,
            real_name: format!("{}", old_fn)
        });
    });
    // println!("{:?}", syntax::print::pprust::item_to_string(&*fn_));
    push(Annotatable::Item(fn_));
}

fn mk_null_ptr(builder: &AstBuilder) -> P<Expr> {
    builder.expr().call()
        .path().global().ids(&["std", "ptr", "null_mut"]).build()
    .build()
}

fn mk_zend_function_entry(builder: &AstBuilder, func_param: Option<&RegisteredFunc>) -> P<Expr> {
    let name_expr;
    let handler_expr;

    match func_param {
        None => {
            name_expr = mk_null_ptr(builder);
            handler_expr = builder.expr().none();
        },
        Some(func) => {
            name_expr = builder.expr().method_call("as_ptr").str(intern(&func.real_name)).build();
            handler_expr = builder.expr().some().id(&func.internal_name);
        }
    }
    builder.expr().struct_()
        .id("ZendFunctionEntry").build()
        .field("name").build(name_expr)
        .field("handler").build(handler_expr)
        .field("arg_info").build(mk_null_ptr(builder))
        .field("num_args").build(builder.expr().u32(0))
        .field("flags").build(builder.expr().u32(0))
        .build()
}

fn get_php_funcs<'cx>(_: &'cx mut ExtCtxt, _: Span, _: &[TokenTree]) -> Box<MacResult + 'cx> {
    let mut funcs: Option<Vec<RegisteredFunc>> = None;
    REGISTERED_FUNCS.with(|rf| {
        let fn_data = &*rf.borrow();
        let mut fns = Vec::with_capacity(fn_data.len());
        for fn_ in fn_data {
            fns.push(fn_.clone());
        }
        funcs = Some(fns)
    });
    
    let builder = AstBuilder::new();
    let mut expr = builder.expr().slice();
    for func in funcs.unwrap() {
        expr = expr.expr().build(mk_zend_function_entry(&builder, Some(&func)));
    }
    
    let expr_ = expr.expr().build(mk_zend_function_entry(&builder, None)).build();
    // println!("{:?}", syntax::print::pprust::expr_to_string(&expr_));
    MacEager::expr(expr_)
}
