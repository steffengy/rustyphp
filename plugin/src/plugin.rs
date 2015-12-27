#![feature(plugin_registrar, rustc_private)]

extern crate aster;
extern crate syntax;
extern crate rustc_plugin;
extern crate rustyphp_core;

use std::cell::RefCell;

use aster::AstBuilder;
use rustc_plugin::Registry;
use syntax::ast::{Ty, Expr_, Expr, Stmt, Item, Item_, MetaItem, FunctionRetTy, TokenTree};
use syntax::ext::base::{MacResult, MacEager, ExtCtxt};
use syntax::codemap::Span;
use syntax::ext::base::Annotatable;
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::parse::token::intern;
use syntax::ptr::P;

use rustyphp_core::*;

thread_local!(static REGISTERED_FUNCS: RefCell<Vec<RegisteredFunc>> = RefCell::new(vec![]));

//TODO: Replace panic! with span errors where appropriate / error handling with exceptions (PHP_side)

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

fn mk_macro_expr(builder: &AstBuilder, mac_item: P<Item>) -> P<Expr> {
    let mac = match mac_item.node {
        Item_::ItemMac(ref mac) => mac.clone(),
        _ => panic!("mac building failed"), //This cannot happen
    };
    //TODO: Get integrated into aster
    builder.expr().build_expr_(Expr_::ExprMac(mac))
}

/// Assign the value of "ret" to the return_value zval "zv"
fn build_assign_ret(builder: &AstBuilder, field: Option<&P<Ty>>, src: P<Expr>, target: P<Expr>) -> Vec<P<Stmt>> {
    match field {
        None => vec![],
        Some(_) => {
            let mac_item = builder.item().mac().path().id("zend_try_option").build()
                .expr().method_call("assign_to").build(src).with_arg(target)
                .build()
                .build();
  
            vec![
                builder.stmt()
                    .semi().build(mk_macro_expr(&builder, mac_item))
            ]
        }
    }
}

/// #[php_func] to declare exported php functions
fn expand_php_func(_: &mut ExtCtxt, _: Span, _: &MetaItem, anno: &Annotatable, push: &mut FnMut(Annotatable)) {
    let builder = AstBuilder::new();

    // Gather information about the original function
    // like return_value type and params
    let mut fn_arguments = None;
    let return_type;
    let orig_fn_item;

    match anno {
        &Annotatable::Item(ref item) => {
            orig_fn_item = item;
            match item.node {
                Item_::ItemFn(ref fn_decl, _, _, _, _, _) => {
                    //println!("{:?}", fn_decl.inputs);
                    // Function arguments
                    let mut fn_args = Vec::with_capacity(fn_decl.inputs.len());
                    for arg in &fn_decl.inputs {
                        fn_args.push(&arg.ty.node);
                    }
                    fn_arguments = Some(fn_args);
                    // Return Value
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

    // Call the old function (wrapper call)
    let mut fn_expr_args = vec![];
    match fn_arguments {
        None => {},
        Some(args) => {
            // TODO: verify arg_count >= required_args
            for (i, _) in args.iter().enumerate() {
                let mac_item = builder.item().mac().path().id("zend_try").build()
                    .expr().call()
                    .path().ids(&["From", "from"]).build()
                .with_arg(
                    builder
                        .expr().method_call("arg")
                        .build(builder.expr().id("ex"))
                        .with_arg(builder.expr().usize(i))
                        .build()
                )
                .build()
                .build();
                fn_expr_args.push(mk_macro_expr(&builder, mac_item));
            }
        }
    }
    // TODO: Generate argument handling code... (call convert or something? builtin)
    let block_builder = builder.block()
        .stmt().let_id("ret").call()
            .path().id(old_fn).build()
            .with_args(fn_expr_args)
            .build();

    let src = builder.expr().path().id("ret").build();
    let target = builder.expr().path().id("zv").build();

    // Assign the return value of the function to the return_value zval
    let block = block_builder
        .with_stmts(build_assign_ret(&builder, return_type, src, target)).build();

    // Generate the function definition of the wrapper func
    let fn_ = builder.item()
        .attr().inline()
        .fn_(new_fn.clone())
            .arg("ex").ty().ref_().mut_().ty().path()
                .global().ids(&["rustyphp", "ExecuteData"]).build() //execute_data as execute_data *
            .arg("zv").ty().ref_().mut_().ty().path()
                .global().ids(&["rustyphp", "Zval"]).build() //return_value as zval *
            .default_return()
            .abi(syntax::abi::Abi::C)
            .build(block);

    // Register the function
    REGISTERED_FUNCS.with(|rf| {
        (*rf.borrow_mut()).push(RegisteredFunc {
            internal_name: new_fn,
            real_name: format!("{}", old_fn)
        });
    });
    // println!("{}", syntax::print::pprust::item_to_string(&*fn_));
    push(Annotatable::Item(fn_));
}

/// Generate a std::ptr::null_mut() expression
fn mk_null_ptr(builder: &AstBuilder) -> P<Expr> {
    builder.expr().call()
        .path().global().ids(&["std", "ptr", "null_mut"]).build()
    .build()
}

/// Generate an AST expr for a function entry for the PHP module export
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

/// Macro: Get a list of registered PHP functions (used to generate the DLL exports)
fn get_php_funcs<'cx>(_: &'cx mut ExtCtxt, _: Span, _: &[TokenTree]) -> Box<MacResult + 'cx> {
    let mut funcs: Option<Vec<RegisteredFunc>> = None;
    REGISTERED_FUNCS.with(|rf| {
        let fn_data = &*rf.borrow();
        funcs = match fn_data.len() {
            0 => None,
            len => Some(fn_data.iter().map(|fn_| fn_.clone()).collect())
        }
    });
    
    let builder = AstBuilder::new();
    let expr = match funcs {
        None => builder.expr().none(),
        Some(funcs) => {
            let mut expr_ = builder.expr().slice();
            for func in funcs {
                expr_ = expr_.expr().build(mk_zend_function_entry(&builder, Some(&func)));
            }
            builder.expr().some().build(builder.expr().call()
                .path().global().ids(&["std", "boxed", "Box", "new"]).build()
                .with_arg(expr_.expr().build(mk_zend_function_entry(&builder, None)).build())
                .build()
            )
        }
    };

    // println!("{}", syntax::print::pprust::expr_to_string(&expr));
    MacEager::expr(expr)
}
