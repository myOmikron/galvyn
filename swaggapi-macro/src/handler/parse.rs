use std::collections::HashMap;

use proc_macro2::Delimiter;
use proc_macro2::Group;
use proc_macro2::Ident;
use proc_macro2::Literal;
use proc_macro2::TokenStream;
use proc_macro2::TokenTree;
use quote::quote;
use quote::quote_spanned;
use syn::parse2;
use syn::ItemFn;

pub fn parse(args: TokenStream, item: TokenStream) -> Result<(Args, ItemFn), TokenStream> {
    match parse2(item) {
        Ok(item) => Ok((parse_args(args)?, item)),
        Err(err) => Err(err.into_compile_error()),
    }
}

pub struct Args {
    pub positional: Vec<TokenTree>,
    pub keyword: HashMap<Ident, TokenTree>,
}
fn parse_args(args: TokenStream) -> Result<Args, TokenStream> {
    let mut args_iter = args.clone().into_iter();
    enum Arg {
        Pos(TokenTree),
        Key(Ident, TokenTree),
    }
    let mut args_vec = Vec::new();
    loop {
        let Some(first) = args_iter.next() else {
            break;
        };

        let Some(punct) = args_iter.next() else {
            args_vec.push(Arg::Pos(first));
            break;
        };

        match punct {
            TokenTree::Punct(punct) if punct.as_char() == ',' => {
                args_vec.push(Arg::Pos(first));
                continue;
            }
            TokenTree::Punct(punct) if punct.as_char() == '=' => {
                let TokenTree::Ident(first) = first else {
                    return Err(quote_spanned!(first.span()=>
                        compile_error!(concat!("expected identifier got `", stringify!(#first), "`"));
                    ));
                };

                let Some(second) = args_iter.next() else {
                    return Err(quote_spanned! {punct.span()=>
                        compile_error!("missing value");
                    });
                };

                args_vec.push(Arg::Key(first, second));
            }
            TokenTree::Group(group) if matches!(group.delimiter(), Delimiter::Parenthesis) => {
                let TokenTree::Ident(first) = first else {
                    return Err(quote_spanned!(first.span()=>
                        compile_error!(concat!("expected identifier got `", stringify!(#first), "`"));
                    ));
                };

                args_vec.push(Arg::Key(
                    first,
                    TokenTree::Group(Group::new(Delimiter::Bracket, group.stream())),
                ));
            }
            _ => {
                return Err(quote_spanned! {punct.span()=>
                    compile_error!(concat!("expected `,`, `=` or `(` got `", stringify!(#punct), "`"));
                })
            }
        }

        match args_iter.next() {
            None => {
                break;
            }
            Some(TokenTree::Punct(punct)) if punct.as_char() == ',' => {
                continue;
            }
            Some(token) => {
                return Err(quote_spanned!(token.span()=>
                    compile_error!(concat!("expected `,` got `", stringify!(#token), "`"));
                ))
            }
        }
    }

    let mut positional = Vec::new();
    let mut keyword = HashMap::new();
    let mut duplicate = false;
    let mut wrong_order = false;
    for arg in args_vec {
        match arg {
            Arg::Pos(arg) => {
                positional.push(arg);
                if !keyword.is_empty() {
                    wrong_order = true;
                }
            }
            Arg::Key(key, val) => {
                if keyword.insert(key, val).is_some() {
                    duplicate = true;
                }
            }
        }
    }

    if duplicate || wrong_order {
        let mut format_str = String::with_capacity((positional.len() + keyword.len()) * 2);
        while format_str.len() < format_str.capacity() {
            format_str.push_str("{}");
        }
        let format_str = Literal::string(&format_str);
        return Err(quote!(
            const _: fn() = || {
                format!(#format_str, #args):
            };
        ));
    }

    Ok(Args {
        positional,
        keyword,
    })
}
