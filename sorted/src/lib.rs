use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Item, parse_macro_input};
use syn::spanned::Spanned;

fn check_enum_order(args: &TokenStream, input: &Item) -> syn::Result<()>{
    let mut idents = vec![];
    match input{
        Item::Enum(ref item) =>{
            for variant in item.variants.iter() {
                idents.push(&variant.ident);
            }
            check_order(idents)
        },
        _=>{
            Err(syn::Error::new(args.span(), "expected enum or match expression"))
        }
    }
}

fn check_order(idents: Vec<&Ident>)-> syn::Result<()> {
    if idents.is_empty(){
        return Ok(());
    }
    let mut sorted = idents.clone();
    sorted.sort_by(|ident1, ident2| ident1.cmp(ident2));
    for (a, b) in idents.iter().zip(sorted.iter()) {
        if a.to_string() != b.to_string(){
            let msg = format!("{} should sort before {}", b.to_string(), a.to_string());
            return Err(syn::Error::new(b.span(), msg));
        }
    }

    Ok(())
}


#[proc_macro_attribute]
pub fn sorted(args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Item);
    let args: TokenStream = args.into();
    let mut result = TokenStream::new();
    match check_enum_order(&args, &input) {
        Ok(_) =>{},
        Err(err) => {
            result.extend(err.into_compile_error());
        }
    }
    result.extend(quote! { #input });
    result.into()
}


