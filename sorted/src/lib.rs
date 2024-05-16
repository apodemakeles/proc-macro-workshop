use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Arm, Attribute, Error, ExprMatch, Item, ItemFn, Meta, parse_macro_input, Pat};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;

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

#[derive(Default)]
struct ItemFnVisitor{
    error: Option<syn::Error>
}

impl VisitMut for ItemFnVisitor{
    fn visit_expr_match_mut(&mut self, expr: &mut ExprMatch) {
        if self.error.is_some(){
            return;
        }
        let attrs = remove_sorted_attr(&expr.attrs);
        if expr.attrs.len() != attrs.len(){
            expr.attrs = attrs;
            let idents = extract_arm_idents(&expr.arms);
            if let Err(error) = check_order(idents){
                self.error = Some(error);
            }
        }
    }
}

fn extract_arm_idents(arms: &Vec<Arm>) -> Vec<&Ident> {
    let mut result = vec![];
    for arm in arms {
        match &arm.pat {
            Pat::Path(pat_path) => {
                if let Some(ident) = pat_path.path.get_ident() {
                    result.push(ident);
                }
            }
            Pat::TupleStruct(pat_tuple) => {
                if let Some(ident) = pat_tuple.path.get_ident() {
                    result.push(ident);
                }
            }
            _ => {}
        }
    }
    result
}

fn remove_sorted_attr(attrs: &[Attribute]) -> Vec<Attribute>{
    attrs.iter().filter(|attr|{
        match &attr.meta {
            Meta::Path(path)=>{
                if let Some(ident) = path.get_ident(){
                    if ident.to_string() == "sorted"{
                        return false
                    }
                }
            }
            _=>{}
        }
        true
    }).map(Attribute::clone).collect()
}

#[proc_macro_attribute]
pub fn check(_args: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);
    let mut visitor = ItemFnVisitor::default();
    visitor.visit_item_fn_mut(&mut item_fn);

    let mut result = TokenStream::new();
    match visitor.error {
        None => {}
        Some(err) => { result.extend(err.into_compile_error()); }
    }
    result.extend(quote! { #item_fn });
    result.into()
}

