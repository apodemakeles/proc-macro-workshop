use proc_macro2::TokenStream;
use quote::quote;
use syn::{Item, parse_macro_input};
use syn::spanned::Spanned;

fn check(args: TokenStream, input: Item) -> TokenStream{
    match input{
        Item::Enum(ref item) =>{
            let mut names: Vec<String> = vec![];
            for variant in item.variants.iter() {
                let cur_name = variant.ident.to_string();
                for name in names.iter() {
                    if &cur_name < name{
                        let msg = format!("{} should sort before {}", cur_name, name);
                        return syn::Error::new(variant.span(), msg).into_compile_error();
                    }
                }
                names.push(cur_name);
            }
            return quote! { #input }
        },
        _=>{
            return syn::Error::new(args.span(), "expected enum or match expression").into_compile_error();
        }
    }
}


#[proc_macro_attribute]
pub fn sorted(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Item);
    let args: TokenStream = args.into();
    check(args.into(), input).into()
}


