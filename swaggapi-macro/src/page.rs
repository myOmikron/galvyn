use std::collections::HashMap;

use proc_macro2::Ident;
use proc_macro2::Literal;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use quote::ToTokens;
use syn::bracketed;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse2;
use syn::MetaList;
use syn::Token;
use syn::Visibility;

pub fn page(input: TokenStream) -> TokenStream {
    match parse2::<Page>(input) {
        Ok(page) => page.into_token_stream(),
        Err(err) => err.into_compile_error(),
    }
}

struct Page {
    ident: Ident,
    kwargs: HashMap<Ident, TokenTree>,
}

impl Parse for Page {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut kwargs = HashMap::new();
        while input.peek(Token![#]) {
            input.parse::<Token![#]>()?;
            let content;
            bracketed!(content in input);

            if content.peek(kw::page) {
                let meta: MetaList = content.parse()?;
                meta.parse_nested_meta(|nested| {
                    let key = nested.path.require_ident()?;
                    if kwargs.contains_key(key) {
                        return Err(syn::Error::new_spanned(key, "Duplicate key"));
                    }

                    let value = nested.value()?.parse()?;
                    kwargs.insert(key.clone(), value);

                    Ok(())
                })?;
            } else {
                // We have to drain `content` because syn would otherwise error with "unexpected token".
                while !content.is_empty() {
                    let _: TokenTree = content.parse()?;
                }
            }
        }

        input.parse::<Visibility>()?;
        input.parse::<Token![struct]>()?;
        let ident = input.parse::<Ident>()?;
        input.parse::<Token![;]>()?;

        kwargs
            .entry(Ident::new("filename", Span::call_site()))
            .or_insert_with(|| TokenTree::Literal(Literal::string(&format!("{ident}.json"))));

        Ok(Self { ident, kwargs })
    }
}

impl ToTokens for Page {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { ident, kwargs } = self;
        let keys = kwargs.keys();
        let values = kwargs.values();
        tokens.extend(quote! {
            impl ::swaggapi::internals::AccessSwaggapiPageBuilder for #ident {
                fn get_builder(&self) -> &'static ::swaggapi::SwaggapiPageBuilder {
                    static BUILDER: ::swaggapi::SwaggapiPageBuilder = ::swaggapi::SwaggapiPageBuilder::new()
                    #(
                        .#keys(#values)
                    )*
                    ;
                    &BUILDER
                }
            }
        });
    }
}

mod kw {
    syn::custom_keyword!(page);
}
