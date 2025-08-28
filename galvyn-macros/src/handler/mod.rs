use std::str::FromStr;

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::format_ident;
use quote::quote;
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::FnArg;
use syn::ItemFn;
use syn::Meta;
use syn::MetaNameValue;
use syn::ReturnType;
use syn::Type;

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
    let core_crate = match keyword.remove(&Ident::new("core_crate", Span::call_site())) {
        None => quote! { ::galvyn::core },
        Some(value) => {
            let literal = match &value {
                TokenTree::Literal(literal) => Some(literal.to_string()),
                _ => None,
            };
            let Some(literal) = literal
                .as_ref()
                .and_then(|s| s.strip_suffix('"'))
                .and_then(|s| s.strip_prefix('"'))
            else {
                let err = quote_spanned! {value.span()=>
                    compile_error!("Expected string literal");
                };
                return quote! {
                    #err
                    #tokens
                };
            };

            let Ok(path) = TokenStream::from_str(literal) else {
                let err = quote_spanned! {value.span()=>
                    compile_error!("Expected crate path");
                };
                return quote! {
                    #err
                    #tokens
                };
            };
            path
        }
    };

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

    let args_todo = sig
        .inputs
        .iter()
        .map(|_| quote! {todo!()})
        .collect::<Vec<_>>();

    let func_ident = &sig.ident;
    let module_ident = format_ident!("__{func_ident}_module");
    let marker_ident = format_ident!("__{func_ident}_marker");

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
            #core_crate::get_metadata!(
                #core_crate::handler::request_part::RequestPartMetadata,
                #part
            )
        }
    });

    let request_body = if let Some(body) = request_types.last() {
        quote_spanned! {body.span()=>
            #core_crate::get_metadata!(
                #core_crate::handler::request_body::RequestBodyMetadata,
                #body
            )
        }
    } else {
        quote! { None }
    };

    let response_types = match &sig.output {
        ReturnType::Default => Vec::new(),
        ReturnType::Type(_, return_type) => match return_type.as_ref() {
            Type::Tuple(tuple) => tuple.elems.iter().collect(),
            return_type => vec![return_type],
        },
    };

    let response_modifier = if let Some(body) = response_types.first() {
        quote_spanned! {body.span()=>
            #core_crate::get_metadata!(
                #core_crate::handler::ResponseModifier,
                #body
            )
        }
    } else {
        quote! { None }
    };

    let response_parts = response_types.iter().map(|part| {
        quote_spanned! {part.span()=>
            #core_crate::get_metadata!(
                #core_crate::handler::response_part::ResponsePartMetadata,
                #part
            )
        }
    });

    let response_body = if let Some(body) = response_types.last() {
        quote_spanned! {body.span()=>
            #core_crate::get_metadata!(
                #core_crate::handler::response_body::ResponseBodyMetadata,
                #body
            )
        }
    } else {
        quote! { None }
    };

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

    let (impl_generics, type_generics, where_clause) = sig.generics.split_for_impl();
    let turbo_fish = type_generics.as_turbofish();
    let type_params = sig.generics.type_params().map(|param| &param.ident);

    let declaration = if sig.generics.params.is_empty() {
        // "Normal" case of a non-generic handler
        //
        // We can emit a basic unit struct which is easier to understand
        // (for tooling) than the general case below.
        quote! {
            #[allow(non_camel_case_types)]
            pub struct #func_ident;

            impl #impl_generics Default for #func_ident #type_generics #where_clause {
                fn default() -> Self {
                    Self
                }
            }
        }
    } else {
        // "Special" case if a generic handler
        //
        // We would like to emit a unit struct but rust does not support generic unit structs.
        // (The builtin PhantomData being the exception.)
        // This workaround is based on a [case study](github.com/dtolnay/case-studies) by dtolnay.
        quote! {
            mod #module_ident {
                pub use self::#func_ident::*;

                #[allow(non_camel_case_types)]
                pub enum #func_ident #impl_generics {
                    #func_ident,

                    #[doc(hidden)]
                    #marker_ident(::std::convert::Infallible, ::std::marker::PhantomData<((), #(#type_params)*)>),
                }
            }
            #vis use #module_ident::*;

            impl #impl_generics Default for #func_ident #type_generics #where_clause {
                fn default() -> Self {
                    Self::#func_ident
                }
            }
        }
    };
    quote! {
        #[allow(missing_docs, clippy::missing_docs_in_private_items)]
        #declaration

        impl #impl_generics Clone for #func_ident #type_generics #where_clause {
            fn clone(&self) -> Self {
                *self
            }
        }
        impl #impl_generics Copy for #func_ident #type_generics #where_clause {}
        impl #impl_generics #core_crate::handler::GalvynHandler for #func_ident #type_generics #where_clause {
            fn meta(&self) -> #core_crate::handler::HandlerMeta {
                #core_crate::handler::HandlerMeta {
                    method: #core_crate::re_exports::axum::http::method::Method::#method,
                    path: #path,
                    deprecated: #deprecated,
                    doc: &[#(
                        #doc,
                    )*],
                    ident: stringify!(#func_ident),
                    request_parts: {
                        let mut x = ::std::vec::Vec::new();
                        #(
                            ::std::iter::Extend::extend(&mut x, #request_parts);
                        )*
                        x
                    },
                    request_body: #request_body,
                    response_modifier: #response_modifier,
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
            fn method_router(&self) -> #core_crate::re_exports::axum::routing::MethodRouter {
                #tokens

                #[allow(unreachable_code, clippy::diverging_sub_expression)]
                let _ = async {
                    // Future must implement Sent
                    #core_crate::macro_utils::test_trait::send(#func_ident #turbo_fish(#(#args_todo),*));

                    // Arguments must implement Sent
                    #(
                        #core_crate::macro_utils::test_trait::send::<#request_types>(panic!());
                    )*

                    // Return must implement IntoResponse
                    #core_crate::macro_utils::test_trait::into_response(#func_ident #turbo_fish(#(#args_todo),*).await);
                };


                #core_crate::re_exports::axum::routing::MethodRouter::new()
                    .on(#core_crate::re_exports::axum::routing::MethodFilter::#method, #func_ident #turbo_fish)
            }
        }
    }
}
