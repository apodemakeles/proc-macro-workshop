use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::{Expr, LitInt, parse_macro_input, Token};

struct Seq{
}

impl Parse for Seq{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _number: Ident = input.parse()?;
        let _ = input.parse::<Token![in]>()?;
        let _start: LitInt = input.parse()?;
        let _ = input.parse::<Token![..]>()?;
        let _end: LitInt = input.parse()?;
        let _ = input.parse::<Token![..]>()?;
        let _: Expr = input.parse()?;

        Ok(Seq{})
    }
}

#[proc_macro]
pub fn seq(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Seq);

    proc_macro::TokenStream::new()
}