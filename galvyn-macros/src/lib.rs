mod handler;

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), None).into()
}

#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("GET")).into()
}

#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("POST")).into()
}

#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("PUT")).into()
}

#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("DELETE")).into()
}

#[proc_macro_attribute]
pub fn head(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("HEAD")).into()
}

#[proc_macro_attribute]
pub fn options(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("OPTIONS")).into()
}

#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("PATCH")).into()
}

#[proc_macro_attribute]
pub fn trace(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("TRACE")).into()
}

#[proc_macro_derive(Model, attributes(rorm))]
pub fn derive_rorm_model(input: TokenStream) -> TokenStream {
    rorm_macro_impl::derive_model(
        input.into(),
        rorm_macro_impl::MacroConfig {
            rorm_path: quote! { ::galvyn::rorm },
            ..Default::default()
        },
    )
    .into()
}

#[proc_macro_derive(Patch, attributes(rorm))]
pub fn derive_rorm_patch(input: TokenStream) -> TokenStream {
    rorm_macro_impl::derive_patch(
        input.into(),
        rorm_macro_impl::MacroConfig {
            rorm_path: quote! { ::galvyn::rorm },
            ..Default::default()
        },
    )
    .into()
}

#[proc_macro_derive(DbEnum, attributes(rorm))]
pub fn derive_rorm_db_enum(input: TokenStream) -> TokenStream {
    rorm_macro_impl::derive_db_enum(
        input.into(),
        rorm_macro_impl::MacroConfig {
            rorm_path: quote! { ::galvyn::rorm },
            ..Default::default()
        },
    )
    .into()
}
