// #![feature(proc_macro_diagnostic)]

use proc_macro2::{Group, Ident, Literal, TokenStream, TokenTree};
use syn::{braced, LitInt, parse_macro_input, Token};
use syn::parse::{Parse, ParseStream};

struct Seq{
    number: Ident,
    start: isize,
    end: isize,
    body: TokenStream,
}

impl Seq {
    fn replace_number(&self, tokens: TokenStream, n: isize) -> TokenStream {
        tokens.into_iter().map(|node| {
            match node {
                TokenTree::Ident(ident) if ident.to_string() == self.number.to_string() => {
                    TokenTree::Literal(Literal::isize_unsuffixed(n))
                },
                TokenTree::Group(group) =>{
                    TokenTree::Group(Group::new(group.delimiter(), self.replace_number(group.stream(), n)))
                }
                _ => node
            }
        }).collect()
    }

    fn expand(&self) -> TokenStream{
        let mut result = TokenStream::new();
        for i in self.start..self.end {
            let body = self.replace_number(self.body.clone(), i);
            result.extend(body);
        }

        result
    }
}

impl Parse for Seq{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let number: Ident = input.parse()?;
        let _ = input.parse::<Token![in]>()?;
        let start: LitInt = input.parse()?;
        let _ = input.parse::<Token![..]>()?;
        let end: LitInt = input.parse()?;

        let body;
        braced!(body in input);

        let body: TokenStream = body.parse()?;

        Ok(Seq{
            number,
            start: start.base10_parse()?,
            end: end.base10_parse()?,
            body
        })
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Seq);
    input.expand().into()
}