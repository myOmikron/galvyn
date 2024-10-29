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
use quote::ToTokens;
use syn::FnArg;
use syn::ItemFn;
use syn::Meta;
use syn::MetaNameValue;
use syn::ReturnType;

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
    let argument_type = sig
        .inputs
        .iter()
        .filter_map(|arg| match arg {
            FnArg::Receiver(_) => None,
            FnArg::Typed(arg) => Some(&arg.ty),
        })
        .collect::<Vec<_>>();
    let return_type = match sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, return_type) => return_type.into_token_stream(),
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
        #[allow(non_upper_case_globals, missing_docs)]
        #vis static #func_ident: ::swaggapi::internals::SwaggapiHandler =  {
            #tokens

            const N: usize =  0 #(+ {let _ = stringify!(#argument_type); 1})*;
            static FNS: [::std::option::Option<::swaggapi::handler_argument::HandlerArgumentFns>; N] = [#(
                ::swaggapi::handler_argument::macro_helper::get_handler_argument_fns(
                    || ::swaggapi::handler_argument::macro_helper::TraitProbe::<#argument_type>::new().get_handler_argument(),
                    || ::swaggapi::handler_argument::macro_helper::TraitProbe::<#argument_type>::new().is_handler_argument(),
                ),
            )*];
            const _: () = {#(
                ::swaggapi::handler_argument::macro_helper::check_handler_argument(
                    || ::swaggapi::handler_argument::macro_helper::TraitProbe::<#argument_type>::new().get_handler_argument()
                );
            )*};

            ::swaggapi::internals::SwaggapiHandler {
                method: ::swaggapi::internals::HttpMethod::#method,
                path: #path,
                deprecated: #deprecated,
                doc: &[#(
                    #doc,
                )*],
                ident: #ident,
                tags: &#tags,
                responses: <#return_type as ::swaggapi::as_responses::AsResponses>::responses,
                handler_arguments: &FNS,
                method_router: || ::swaggapi::re_exports::axum::routing::MethodRouter::new()
                    .on(::swaggapi::internals::HttpMethod::#method.axum(), #func_ident),
            }

        };
    }
}
