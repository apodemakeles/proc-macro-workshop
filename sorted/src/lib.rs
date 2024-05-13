use proc_macro2::TokenStream;
use quote::quote;
use syn::{Item, parse_macro_input};
use syn::spanned::Spanned;

#[proc_macro_attribute]
pub fn sorted(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Item);
    let args: TokenStream = args.into();
    match input{
        Item::Enum(ref _item) =>{},
        _=>{
            return syn::Error::new(args.span(), "expected enum or match expression").into_compile_error().into();
        }
    }
    let tokens = quote! {
        #input
    };

    tokens.into()
}
