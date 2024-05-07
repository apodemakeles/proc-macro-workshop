use proc_macro2::{Ident, TokenStream};
use syn::parse::{Parse, ParseStream};
use syn::{braced, Expr, LitInt, parse_macro_input, Token};

struct Seq{
    number: Ident,
    start: LitInt,
    end: LitInt,
    body: TokenStream,
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
            start,
            end,
            body
        })
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Seq);

    proc_macro::TokenStream::new()
}