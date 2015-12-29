#![feature(plugin_registrar, rustc_private)]

extern crate aster;
extern crate syntax;
extern crate rustc_plugin;
extern crate rustyphp;

use std::cell::RefCell;

use aster::AstBuilder;
use aster::ident::ToIdent;
use rustc_plugin::Registry;
use syntax::ast::{Arg, Expr_, Expr, FunctionRetTy, Ident, Item, Item_, MacStmtStyle, MetaItem, MutTy, Mutability, Pat_, Stmt, Stmt_, Ty, Ty_, TokenTree};
use syntax::ext::base::{MacResult, MacEager, ExtCtxt};
use syntax::codemap::Span;
use syntax::ext::base::Annotatable;
use syntax::ext::base::SyntaxExtension::MultiDecorator;
use syntax::parse::token::Token;
use syntax::parse::token::intern;
use syntax::ptr::P;
use syntax::util::small_vector::SmallVector;

thread_local!(static ALREADY_COMPILED: RefCell<bool> = RefCell::new(false));
thread_local!(static REGISTERED_FUNCS: RefCell<Vec<RegisteredFunc>> = RefCell::new(vec![]));

//TODO: Replace panic! with span errors

#[derive(Debug, Clone)]
struct RegisteredFunc {
    /// The internal name (mostly zif_ prefix for the C-definition)
    internal_name: String,
    /// The path to the func (mod-wise)
    mod_path: Vec<Ident>,
    /// The real name of the func
    real_name: String,
    /// The parameters a function can take
    args: Option<Vec<Arg>>,
    required_args: usize
}

#[plugin_registrar]
pub fn registrar(reg: &mut Registry) {
    reg.register_syntax_extension(intern("php_func"), MultiDecorator(Box::new(expand_php_func)));
    reg.register_macro("get_php_funcs", get_php_funcs);
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
fn expand_php_func(ectx: &mut ExtCtxt, span: Span, _: &MetaItem, anno: &Annotatable, push: &mut FnMut(Annotatable)) {
    ALREADY_COMPILED.with(|rf| {
        let already_compiled = rf.borrow();
        if *already_compiled {
            ectx.span_err(span, "`php_func` cannot be used outside an extension. Make sure the `php_ext` macro is resolved BEFORE.");
        }
    });
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
                    // Function arguments
                    let mut fn_args = Vec::with_capacity(fn_decl.inputs.len());
                    for arg in &fn_decl.inputs {
                        fn_args.push(arg.clone());
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
    let mut required_args = 0;
    let mut block_builder = builder.block();

    match fn_arguments {
        None => {},
        Some(ref args) => {
            for arg in args {
                required_args += 1;
            }
            let dummy_span = builder.expr().id(",").span;
            if required_args > 0 {
                // TODO: verify arg_count >= required_args
                block_builder = block_builder.stmt().expr().build(mk_macro_expr(&builder,
                    builder.item().mac().path().id("verify_arg_count").build()
                    .with_arg(builder.expr().lit().str(old_fn))
                    .with_arg(TokenTree::Token(dummy_span, Token::Comma))
                    .with_arg(builder.id("_ex"))
                    .with_arg(TokenTree::Token(dummy_span, Token::Comma))
                    .with_arg(builder.expr().usize(required_args))
                    .build()
                ));
            }
            for (i, _) in args.iter().enumerate() {
                let mac_item = builder.item().mac().path().id("zend_try").build()
                    .expr().call()
                    .path().ids(&["From", "from"]).build()
                .with_arg(
                    builder
                        .expr().method_call("arg")
                        .id("_ex")
                        .with_arg(builder.expr().usize(i))
                        .build()
                )
                .build()
                .build();
                fn_expr_args.push(mk_macro_expr(&builder, mac_item));
            }
        }
    }
    // Variables prefixed with _ since we do (and sometimes cannot) check if they actually are used
    // and else we get plenty ugly warnings
    block_builder = block_builder
        .stmt().let_id("_ret").call()
            .path().id(old_fn).build()
            .with_args(fn_expr_args)
            .build();

    let src = builder.expr().path().id("_ret").build();
    let target = builder.expr().path().id("_zv").build();

    // Assign the return value of the function to the return_value zval
    let block = block_builder
        .with_stmts(build_assign_ret(&builder, return_type, src, target)).build();

    // Generate the function definition of the wrapper func
    let fn_ = builder.item()
        .pub_()
        .attr().inline()
        .fn_(new_fn.clone())
            .arg("_ex").ty().ref_().mut_().ty().path()
                .global().ids(&["rustyphp", "types", "execute_data", "ExecuteData"]).build() //execute_data as execute_data *
            .arg("_zv").ty().ref_().mut_().ty().path()
                .global().ids(&["rustyphp", "Zval"]).build() //return_value as zval *
            .default_return()
            .abi(syntax::abi::Abi::C)
            .build(block);

    // Register the function
    REGISTERED_FUNCS.with(|rf| {
        (*rf.borrow_mut()).push(RegisteredFunc {
            internal_name: new_fn,
            mod_path: ectx.mod_path(),
            real_name: format!("{}", old_fn),
            args: fn_arguments,
            required_args: required_args
        });
    });
    // println!("{}", syntax::print::pprust::item_to_string(&*fn_));
    push(Annotatable::Item(fn_));
}

//TODO: integrate into aster
fn mk_macro_expr(builder: &AstBuilder, mac_item: P<Item>) -> P<Expr> {
    let mac = match mac_item.node {
        Item_::ItemMac(ref mac) => mac.clone(),
        _ => panic!("mac building failed"), //This cannot happen
    };
    // println!("{:?}", mac);
    builder.expr().build_expr_(Expr_::ExprMac(mac))
}

/// Generate a std::ptr::null_mut() expression
fn mk_null_ptr(builder: &AstBuilder) -> P<Expr> {
    builder.expr().call()
        .path().global().ids(&["std", "ptr", "null_mut"]).build()
    .build()
}

fn mk_cast_expr(builder: &AstBuilder, from: P<Expr>, to: P<Ty>) -> P<Expr> {
    builder.expr().build_expr_(Expr_::ExprCast(from, to))
}

fn mk_ty_ptr(builder: &AstBuilder, ty: P<Ty>, mut_: Mutability) -> P<Ty> {
    builder.ty().build_ty_(Ty_::TyPtr(MutTy { ty: ty, mutbl: mut_ }))
}

fn mk_ty_sized_slice(builder: &AstBuilder, ty: P<Ty>, expr: P<Expr>) -> P<Ty> {
    builder.ty().build_ty_(Ty_::TyFixedLengthVec(ty, expr))
}

/// Generate an AST expr for a function entry for the PHP module export
fn mk_zend_function_entry(builder: &AstBuilder, func_param: Option<&RegisteredFunc>) -> P<Expr> {
    let name_expr;
    let handler_expr;
    let arginfo_expr;
    let num_args;

    match func_param {
        None => {
            name_expr = mk_null_ptr(builder);
            arginfo_expr = mk_null_ptr(builder);
            handler_expr = builder.expr().none();
            num_args = 0;
        },
        Some(func) => {
            //TODO: get ExprCast, ptr into aster
            num_args = match func.args {
                None => 0,
                Some(ref x) => x.len() as u32
            };
            let name: Vec<_> = func.real_name.as_bytes().iter().chain(&[0u8]).cloned().collect();
            name_expr = mk_lit_ptr_expr(&builder, name);
            arginfo_expr = builder.expr().block().unsafe_().expr().build(mk_cast_expr(&builder, mk_cast_expr(&builder,
                builder.expr().addr_of().id(format!("ARG_INFO_{}", func.real_name)),
                mk_ty_ptr(&builder, builder.ty().infer(), Mutability::MutImmutable)
            ), mk_ty_ptr(&builder, builder.ty().infer(), Mutability::MutMutable)));
            handler_expr = builder.expr().some()
                //skip 1 path item to ensure we do not try ::crate::mod which fails since ::crate doesn't work within the same crate
                .path().global().ids(func.mod_path.iter().skip(1).chain(&[builder.id(&func.internal_name)])).build()
        }
    }
    builder.expr().struct_()
        .id("ZendFunctionEntry").build()
        .field("name").build(name_expr)
        .field("handler").build(handler_expr)
        .field("arg_info").build(arginfo_expr)
        .field("num_args").build(builder.expr().u32(num_args))
        .field("flags").build(builder.expr().u32(0))
        .build()
}

// TODO: missing in aster: static NAME: ty = val;
fn mk_static_item<T: ToIdent>(builder: &AstBuilder, name: T, ty: P<Ty>, mut_: Mutability, val: P<Expr>) -> P<Item> {
    builder.item().build_item_(name, Item_::ItemStatic(ty, mut_, val))
}

fn mk_lit_ptr_expr(builder: &AstBuilder, bytes: Vec<u8>) -> P<Expr> {
    mk_cast_expr(&builder,
        builder.expr().lit().byte_str(bytes),
        mk_ty_ptr(&builder, builder.ty().infer(), Mutability::MutImmutable)
    )
}

/// Macro: Get a list of registered PHP functions (used to generate the DLL exports)
fn get_php_funcs<'cx>(ectx: &'cx mut ExtCtxt, span: Span, _: &[TokenTree]) -> Box<MacResult + 'cx> {
    let mut funcs: Vec<RegisteredFunc> = vec![];
    REGISTERED_FUNCS.with(|rf| {
        let fn_data = &*rf.borrow();
        funcs = fn_data.iter().map(|fn_| fn_.clone()).collect();
    });

    // we do not really check if it's actually length thats asked yet
    let builder = AstBuilder::new();

    // We assume that this is only used on module initialization, so we use this to trigger
    // our module initialized state
    ALREADY_COMPILED.with(|rf| {
        let mut already_compiled = rf.borrow_mut();
        if *already_compiled {
            ectx.span_err(span, "`get_php_funcs` cannot be called multiple times since it's used to detect multiple php_ext declarations by `php_ext`");
        }
        *already_compiled = true;
    });
    let mut expr_ = builder.expr().slice();
    let func_slice_len = funcs.len() + 1;

    let mut item_vec = Vec::with_capacity(func_slice_len);
    for func in funcs {
        expr_ = expr_.expr().build(mk_zend_function_entry(&builder, Some(&func)));
        // Build argument info
        // TODO: Make name generation include mod (ensure unique)
        // static MUT FUNC_ARG_INFO: ZendInternalArgInfo = {...}
        let arginfo_path = builder.path().id("ZendInternalArgInfo").build();

        let mut slice_builder = builder
            .expr().slice()
            // Header Building
            .expr().struct_path(arginfo_path.clone())
                .field("arg_name").build(mk_cast_expr(&builder, builder.expr().u32(func.required_args as u32), mk_ty_ptr(&builder, builder.ty().u8(), Mutability::MutImmutable)))
                .field("cls_name").build(mk_null_ptr(&builder))
                .field("type_hint").u8(0) //TODO
                .field("pass_by_ref").u8(0)
                .field("allow_null").bool(false)
                .field("is_variadic").bool(false)
                .build();

        if func.args.is_some() {
            for param in func.args.unwrap() {
                let arg_name: Vec<_> = match param.pat.node {
                    Pat_::PatIdent(_, arg_id, _) => arg_id.node.name.as_str().as_bytes().iter().chain(&[0u8]).cloned().collect(),
                    _ => panic!("Unexpected type for arg pattern")
                };
                let name_expr = mk_lit_ptr_expr(&builder, arg_name);
                slice_builder = slice_builder
                    .expr().struct_path(arginfo_path.clone())
                        .field("arg_name").build(name_expr)
                        .field("cls_name").build(mk_null_ptr(&builder))
                        .field("type_hint").u8(0)
                        .field("pass_by_ref").u8(0)
                        .field("allow_null").bool(true)
                        .field("is_variadic").bool(false)
                        .build();
            }
        }
        let arg_info_slice = slice_builder.build();

        item_vec.push(mk_static_item(
            &builder,
            format!("ARG_INFO_{}", func.real_name),
            mk_ty_sized_slice(&builder, builder.ty().build_path(arginfo_path), builder.expr().usize(func.required_args as usize + 1)),
            Mutability::MutMutable,
            arg_info_slice
        ));
    }
    let func_slice_expr = builder.expr().build(expr_.expr().build(mk_zend_function_entry(&builder, None)).build());
    // static mut FUNC_PTR: [$crate::ZendFunctionEntry; func_len] = [...]

    //TODO: get Ty_::TyFixedLengthVec into aster
    let func_ptr_item = mk_static_item(&builder, "FUNC_PTR",
        mk_ty_sized_slice(&builder,
            builder.ty().path().id("ZendFunctionEntry").build(),
            builder.expr().usize(func_slice_len)
        ),
        Mutability::MutMutable,
        func_slice_expr
    );

    // println!("{}", syntax::print::pprust::expr_to_string(&expr));
    item_vec.push(func_ptr_item);

    let items = SmallVector::many(item_vec);
    MacEager::items(items)
}
