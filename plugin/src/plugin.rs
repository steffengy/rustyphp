#![feature(plugin_registrar, rustc_private)]

extern crate aster;
extern crate syntax;
extern crate rustc_plugin;

use std::cell::RefCell;

use aster::AstBuilder;
use rustc_plugin::Registry;
use syntax::ast::{Ty_, Expr_, Block, Expr, Stmt, Item_, MetaItem, PathParameters, FunctionRetTy, TokenTree};
use aster::block::BlockBuilder;
use syntax::ext::base::{MacResult, MacEager, ExtCtxt};
use syntax::codemap::Span;
use syntax::ext::base::Annotatable;
use syntax::ext::base::SyntaxExtension::{MultiModifier, MultiDecorator};
use syntax::parse::token::intern;

use syntax::ptr::P;

thread_local!(static REGISTERED_FUNCS: RefCell<Vec<RegisteredFunc>> = RefCell::new(vec![]));

#[derive(PartialEq)]
enum VarType {
    Unsupported,
    Long,
    Double,
    Boolean,
    Nullable(Box<VarType>),
}

#[repr(u32)]
enum ZvalType {
    Null = 1,
    False = 2,
    True = 3,
    Long = 4,
    Double = 5
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

fn resolve_type(ty: &Ty_) -> VarType {
    let mut _type = VarType::Unsupported;

    match *ty {
        Ty_::TyPath(_, ref path) => {
            if path.segments.len() == 1 {
                match &*path.segments[0].identifier.name.as_str() {
                    "i8" | "i16" | "i32" | "i64" => _type = VarType::Long,
                    "u8" | "u16" | "u32" | "u64" => _type = VarType::Long,
                    "f64" | "f32" => _type = VarType::Double,
                    "bool" => _type = VarType::Boolean,
                    "Option" => {
                        match path.segments[0].parameters {
                            PathParameters::AngleBracketedParameters(ref param_data) => {
                                assert_eq!(param_data.types.len(), 1);
                                _type = VarType::Nullable(Box::new(resolve_type(&param_data.types[0].node)));
                            }, _ => ()
                        }
                    } _ => ()
                }
            }
        }, _ => ()
    }
    _type
}

fn resolve_zend_type(field: &VarType) -> Option<ZvalType> {
    match *field {
        VarType::Long => Some(ZvalType::Long),
        VarType::Double => Some(ZvalType::Double),
        // skip operations which require additional codegen
        VarType::Nullable(_) => None,
        VarType::Boolean => None,
        VarType::Unsupported => panic!("unsupported vartype")
    }
}

/// Assign an expression to a zval
fn mk_assign_zval(builder: &AstBuilder, target: P<Expr>, val_type: ZvalType, val: Option<P<Expr>>) -> Vec<P<Stmt>> {
    let mut statements: Vec<P<Stmt>> = Vec::with_capacity(2);
    let field_value = builder.expr().field("value").build(target.clone());
    let field = match val_type {
        ZvalType::Null | ZvalType::True | ZvalType::False => None,
        ZvalType::Long => Some(field_value),
        ZvalType::Double => Some(builder.expr().block().unsafe_().expr().method_call("as_double_mut").build(field_value).build()),
    };
    if field != None {
        let field_data = builder.expr().field("data").build(field.unwrap());

        let expected_type = match val_type {
            ZvalType::Long => Some(builder.ty().path().global().ids(&["rustyphp", "php_config", "zend_long"]).build()),
            _ => None,
        };
        let cast_expr = match expected_type {
            None => val.unwrap(),
            Some(expected_type) => builder.expr().build_expr_(Expr_::ExprCast(val.unwrap(), expected_type))
        };
        
        statements.push(builder.stmt().semi().build_assign(field_data, cast_expr));
    }
    let field_type = builder.expr().field("u1").build(target);
    statements.push(builder.stmt().semi().build_assign(field_type, builder.expr().u32(val_type as u32)));
    statements
}

/// Assign the value of "ret" to the return_value zval "zv"
fn build_assign_ret(builder: &AstBuilder, field: &VarType, src: P<Expr>, target: P<Expr>) -> Vec<P<Stmt>> {
    let mut zend_type = resolve_zend_type(&field);

    match *field {
        // Resolve a boolean to it's type of either TRUE/FALSE
        VarType::Boolean => vec![
            builder.stmt().semi().if_()
            .eq().build(src.clone()).true_()
            .then().with_stmts(mk_assign_zval(&builder, target.clone(), ZvalType::True, None)).build()
            .else_().with_stmts(mk_assign_zval(&builder, target, ZvalType::False, None)).build()
        ],
        // A value which might be null or an inner type
        VarType::Nullable(ref inner) => {
            zend_type = resolve_zend_type(&inner);
            let unwrap_expr = builder.expr().method_call("unwrap").build(src.clone()).build();

            vec![
                builder.stmt().semi().if_()
                .eq().build(src.clone()).none()
                .then().with_stmts(mk_assign_zval(&builder, target.clone(), ZvalType::Null, None)).build()
                .else_().with_stmts(match zend_type {
                    Some(_) => mk_assign_zval(&builder, target, zend_type.unwrap(), Some(unwrap_expr)),
                    // We might need more code gen for complex types
                    None => build_assign_ret(builder, inner, unwrap_expr, target)
                }).build()
            ]
        },
        _ => {
            mk_assign_zval(&builder, target, zend_type.unwrap(), Some(src))
        }
    }
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
            field = resolve_type(&ty.node);
            assert!(field != VarType::Unsupported, "php_func: Type not supported: {:?}", ty);
        }
    }

    let block_builder = builder.block()
        .stmt().let_id("ret").call()
            .path().id(old_fn).build()
            .build();

    let src = builder.expr().path().id("ret").build();
    let target = builder.expr().path().id("zv").build();
    let block = block_builder
        .with_stmts(build_assign_ret(&builder, &field, src, target)).build();

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
