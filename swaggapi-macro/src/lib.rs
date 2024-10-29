mod handler;
mod page;

use proc_macro::TokenStream;

#[proc_macro_derive(SwaggapiPage, attributes(page))]
pub fn derive_page(input: TokenStream) -> TokenStream {
    page::page(input.into()).into()
}

#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), None).into()
}

#[proc_macro_attribute]
pub fn get(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Get")).into()
}

#[proc_macro_attribute]
pub fn post(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Post")).into()
}

#[proc_macro_attribute]
pub fn put(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Put")).into()
}

#[proc_macro_attribute]
pub fn delete(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Delete")).into()
}

#[proc_macro_attribute]
pub fn head(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Head")).into()
}

#[proc_macro_attribute]
pub fn options(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Options")).into()
}

#[proc_macro_attribute]
pub fn patch(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Patch")).into()
}

#[proc_macro_attribute]
pub fn trace(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::handler(args.into(), input.into(), Some("Trace")).into()
}
