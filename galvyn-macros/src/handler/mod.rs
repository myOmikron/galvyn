use proc_macro2::Delimiter;
use proc_macro2::Group;
use proc_macro2::Ident;
use proc_macro2::Literal;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::ItemFn;
use syn::Meta;
use syn::MetaNameValue;
use syn::ReturnType;
use syn::{FnArg, Type};

mod parse;

pub fn handler(
    args: TokenStream,
    tokens: TokenStream,
    method: Option<&'static str>,
) -> TokenStream {
    let (
        parse::Args {
            positional,
            mut keyword,
        },
        ItemFn {
            attrs,
            vis,
            sig,
            block: _,
        },
    ) = match parse::parse(args, tokens.clone()) {
        Ok(x) => x,
        Err(err) => {
            return quote! {
                #err
                #tokens
            }
        }
    };

    let mut positional = positional.into_iter();
    let method = method
        .map(|str| TokenTree::Ident(Ident::new(str, Span::call_site())))
        .or_else(|| keyword.remove(&Ident::new("method", Span::call_site())))
        .or_else(|| positional.next())
        .unwrap();
    let path = keyword
        .remove(&Ident::new("path", Span::call_site()))
        .or_else(|| positional.next())
        .unwrap();
    let tags = keyword
        .remove(&Ident::new("tags", Span::call_site()))
        .unwrap_or(TokenTree::Group(Group::new(
            Delimiter::Bracket,
            TokenStream::new(),
        )));

    if let Some(value) = positional.next() {
        let err = quote_spanned! {value.span()=>
            compile_error!("Unexpected value");
        };
        return quote! {
            #err
            #tokens
        };
    }

    if let Some(key) = keyword.into_keys().next() {
        let err = quote_spanned! {key.span()=>
            compile_error!("Unknown key");
        };
        return quote! {
            #err
            #tokens
        };
    }

    let func_ident = &sig.ident;

    let request_types = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(arg) => Some(&arg.ty),
        })
        .collect::<Vec<_>>();

    let request_parts = request_types.iter().map(|part| {
        quote_spanned! {part.span()=>
            ::galvyn::swaggapi::get_metadata!(
                ::galvyn::swaggapi::handler::request_part::RequestPartMetadata,
                #part
            )
        }
    });

    let request_body = if let Some(body) = request_types.last() {
        quote_spanned! {body.span()=>
            ::galvyn::swaggapi::get_metadata!(
                ::galvyn::swaggapi::handler::request_body::RequestBodyMetadata,
                #body
            )
        }
    } else {
        quote! { None }
    };

    let response_types = match sig.output {
        ReturnType::Default => Vec::new(),
        ReturnType::Type(_, return_type) => match *return_type {
            Type::Tuple(tuple) => tuple.elems.into_iter().collect(),
            return_type => vec![return_type],
        },
    };

    let response_parts = response_types.iter().map(|part| {
        quote_spanned! {part.span()=>
            ::galvyn::swaggapi::get_metadata!(
                ::galvyn::swaggapi::handler::response_part::ResponsePartMetadata,
                #part
            )
        }
    });

    let response_body = if let Some(body) = response_types.last() {
        quote_spanned! {body.span()=>
            ::galvyn::swaggapi::get_metadata!(
                ::galvyn::swaggapi::handler::response_body::ResponseBodyMetadata,
                #body
            )
        }
    } else {
        quote! { None }
    };

    let ident = Literal::string(&sig.ident.to_string());
    let deprecated = attrs.iter().any(|attr| {
        attr.meta
            .path()
            .get_ident()
            .map(|ident| ident == "deprecated")
            .unwrap_or(false)
    });
    let deprecated = if deprecated {
        format_ident!("true")
    } else {
        format_ident!("false")
    };
    let doc = attrs.iter().filter_map(|attr| match &attr.meta {
        Meta::NameValue(MetaNameValue {
            path,
            eq_token: _,
            value,
        }) => {
            if path.get_ident()? != "doc" {
                None
            } else {
                Some(value)
            }
        }
        _ => None,
    });
    quote! {
        #[allow(non_camel_case_types)]
        #vis struct #func_ident;
        impl ::galvyn::swaggapi::handler::GalvynHandler for #func_ident {
            fn meta(&self) -> ::galvyn::swaggapi::handler::HandlerMeta {
                ::galvyn::swaggapi::handler::HandlerMeta {
                    method: ::galvyn::swaggapi::re_exports::axum::http::method::Method::#method,
                    path: #path,
                    deprecated: #deprecated,
                    doc: &[#(
                        #doc,
                    )*],
                    ident: #ident,
                    tags: &#tags,
                    request_parts: {
                        let mut x = ::std::vec::Vec::new();
                        #(
                            ::std::iter::Extend::extend(&mut x, #request_parts);
                        )*
                        x
                    },
                    request_body: #request_body,
                    response_parts: {
                        let mut x = ::std::vec::Vec::new();
                        #(
                            ::std::iter::Extend::extend(&mut x, #response_parts);
                        )*
                        x
                    },
                    response_body: #response_body,
                }
            }
            fn method_router(&self) -> ::galvyn::swaggapi::re_exports::axum::routing::MethodRouter {
                #tokens

                ::galvyn::swaggapi::re_exports::axum::routing::MethodRouter::new()
                    .on(::galvyn::swaggapi::re_exports::axum::routing::MethodFilter::#method, #func_ident)
            }
        }
    }
}
