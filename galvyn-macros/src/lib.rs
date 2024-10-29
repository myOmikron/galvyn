mod handler;

use proc_macro::TokenStream;


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
