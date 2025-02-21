#![feature(proc_macro_diagnostic)]
use compilation::CompiledRoute;
use parsing::{Method, Route};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use std::collections::HashMap;
use syn::{
    meta::parser,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::{Comma, Slash},
    Attribute, FnArg, GenericArgument, Item, ItemFn, ItemImpl, Lit, LitStr, Meta, MetaNameValue,
    Path, PathArguments, Signature, Type,
};
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

mod compilation;
mod parsing;

/// A macro that generates statically-typed routes for axum handlers.
///
/// # Syntax
/// ```ignore
/// #[route(<METHOD> "<PATH>" [with <STATE>])]
/// ```
/// - `METHOD` is the HTTP method, such as `GET`, `POST`, `PUT`, etc.
/// - `PATH` is the path of the route, with optional path parameters and query parameters,
///     e.g. `/item/:id?amount&offset`.
/// - `STATE` is the type of axum-state, passed to the handler. This is optional, and if not
///    specified, the state type is guessed based on the parameters of the handler.
///
/// # Example
/// ```
/// use axum::extract::{State, Json};
/// use axum_controller_macros::route;
///
/// #[route(GET "/item/:id?amount&offset")]
/// async fn item_handler(
///     id: u32,
///     amount: Option<u32>,
///     offset: Option<u32>,
///     State(state): State<String>,
///     Json(json): Json<u32>,
/// ) -> String {
///     todo!("handle request")
/// }
/// ```
///
/// # State type
/// Normally, the state-type is guessed based on the parameters of the function:
/// If the function has a parameter of type `[..]::State<T>`, then `T` is used as the state type.
/// This should work for most cases, however when not sufficient, the state type can be specified
/// explicitly using the `with` keyword:
/// ```ignore
/// #[route(GET "/item/:id?amount&offset" with String)]
/// ```
///
/// # Internals
/// The macro expands to a function with signature `fn() -> (&'static str, axum::routing::MethodRouter<S>)`.
/// The first element of the tuple is the path, and the second is axum's `MethodRouter`.
///
/// The path and query are extracted using axum's `extract::Path` and `extract::Query` extractors, as the first
/// and second parameters of the function. The remaining parameters are the parameters of the handler.
#[proc_macro_attribute]
pub fn route(attr: TokenStream, mut item: TokenStream) -> TokenStream {
    match _route(attr, item.clone()) {
        Ok(tokens) => tokens.into(),
        Err(err) => {
            let err: TokenStream = err.to_compile_error().into();
            item.extend(err);
            item
        }
    }
}

fn _route(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream2> {
    // Parse the route and function
    let route = syn::parse::<Route>(attr)?;
    let function = syn::parse::<ItemFn>(item)?;

    // Now we can compile the route
    let route = CompiledRoute::from_route(route, &function)?;
    let path_extractor = route.path_extractor();
    let query_extractor = route.query_extractor();
    let query_params_struct = route.query_params_struct();
    let state_type = &route.state;
    let axum_path = route.to_axum_path_string();
    let http_method = route.method.to_axum_method_name();
    let remaining_numbered_pats = route.remaining_pattypes_numbered(&function.sig.inputs);
    let extracted_idents = route.extracted_idents();
    let remaining_numbered_idents = remaining_numbered_pats.iter().map(|pat_type| &pat_type.pat);
    let route_docs = route.to_doc_comments();

    // Get the variables we need for code generation
    let fn_name = &function.sig.ident;
    let fn_output = &function.sig.output;
    let vis = &function.vis;
    let asyncness = &function.sig.asyncness;
    let (impl_generics, ty_generics, where_clause) = &function.sig.generics.split_for_impl();
    let ty_generics = ty_generics.as_turbofish();
    let fn_docs = function
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let (inner_fn_call, method_router_ty) = {
        (
            quote! { ::axum::routing::#http_method(__inner__function__ #ty_generics) },
            quote! { ::axum::routing::MethodRouter },
        )
    };

    // Generate the code
    Ok(quote! {
        #(#fn_docs)*
        #route_docs
        #vis fn #fn_name #impl_generics() -> (&'static str, #method_router_ty<#state_type>) #where_clause {

            #query_params_struct

            #asyncness fn __inner__function__ #impl_generics(
                #path_extractor
                #query_extractor
                #remaining_numbered_pats
            ) #fn_output #where_clause {
                #function

                #fn_name #ty_generics(#(#extracted_idents,)* #(#remaining_numbered_idents,)* ).await
            }

            (#axum_path, #inner_fn_call)
        }
    })
}

#[derive(Debug, Clone, Default)]
struct MyAttrs {
    middlewares: Vec<syn::Expr>,
    path: Option<syn::Expr>,
    state: Option<syn::Expr>,
}

impl Parse for MyAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut path: Option<syn::Expr> = None;
        let mut state: Option<syn::Expr> = None;
        let mut middlewares: Vec<syn::Expr> = Vec::new();

        // parse while stuff returns
        for nv in Punctuated::<MetaNameValue, Token![,]>::parse_terminated(input)?.into_iter() {
            // syn::MetaNameValue::parse(input) {
            let segs = nv.path.segments.clone().into_pairs();
            let seg = segs.into_iter().next().unwrap().into_value();
            let ident = seg.ident;
            match ident.to_string().as_str() {
                "path" => {
                    if path.is_some() {
                        return Err(syn::Error::new_spanned(path, "duplicate `path` attribute"));
                    }
                    path = Some(nv.value);
                    // panic!("{test:?}");
                    // panic!("{nv:?}");
                }
                "state" => {
                    if state.is_some() {
                        return Err(syn::Error::new_spanned(
                            state,
                            "duplicate `state` attribute",
                        ));
                    }
                    state = Some(nv.value);
                }
                "middleware" => middlewares.push(nv.value),
                _ => {
                    panic!("123");
                }
            }
        }
        Ok(Self {
            state,
            path,
            middlewares,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct MyItem<F, _>
where
    F: Fn(_) -> String,
{
    typed_routing_fn: F,
}

impl<F: Fn(A), _> Parse for MyItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {}
}

#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as MyAttrs);
    let item_impl = parse_macro_input!(item as MyItem);
    // Punctuated<>

    panic!("{item_impl:?}");
    for item in item_impl.items.into_iter() {
        proc_macro::Diagnostic::new(proc_macro::Level::Warning, "test").emit();
        if let syn::Item::Macro(inner) = item {
            panic!("{inner:?}")
        }
        panic!("aaa{item:?}");
    }

    panic!("bbb {args:?}");
    // let input = parse_macro_input!(input as ItemImpl);

    // let mut state_type = None;
    // let mut base_path = None;
    // let mut middlewares = Vec::new();

    // todo!()
    // return item;
    TokenStream::new()
}
