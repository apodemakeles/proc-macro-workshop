use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Arm, Attribute, ExprMatch, Item, ItemFn, Meta, parse_macro_input, Pat, Path, PathSegment};
use syn::spanned::Spanned;
use syn::visit_mut::VisitMut;

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

#[derive(Debug, Clone)]
struct Entry{
    name: String,
    span: Span
}

#[derive(Debug, Clone)]
struct PathEntry<'a>{
    name: String,
    path: &'a Path,
}

fn check_order(entries: Vec<Entry>)-> syn::Result<()> {
    if entries.is_empty(){
        return Ok(());
    }
    let mut sorted = entries.clone();
    sorted.sort_by(|e1, e2| e1.name.cmp(&e2.name));
    eprintln!("{:?}", sorted);
    for (a, b) in entries.iter().zip(sorted.iter()) {
        if a.name != b.name{
            let msg = format!("{} should sort before {}", b.name, a.name);
            return Err(syn::Error::new(b.span, msg));
        }
    }

    Ok(())
}

fn check_path_order(entries: Vec<PathEntry>)-> syn::Result<()> {
    if entries.is_empty(){
        return Ok(());
    }
    let mut sorted = entries.clone();
    sorted.sort_by(|e1, e2| e1.name.cmp(&e2.name));
    eprintln!("{:?}", sorted);
    for (a, b) in entries.iter().zip(sorted.iter()) {
        if a.name != b.name{
            let msg = format!("{} should sort before {}", b.name, a.name);
            return Err(syn::Error::new_spanned(b.path, msg));
        }
    }

    Ok(())
}

fn check_enum_order(args: &TokenStream, input: &Item) -> syn::Result<()>{
    let mut idents = vec![];
    match input{
        Item::Enum(ref item) =>{
            for variant in item.variants.iter() {
                idents.push(Entry{name: variant.ident.to_string(), span: variant.span()});
            }
            check_order(idents)
        },
        _=>{
            Err(syn::Error::new(args.span(), "expected enum or match expression"))
        }
    }
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

impl VisitMut for ItemFnVisitor {
    fn visit_expr_match_mut(&mut self, expr: &mut ExprMatch) {
        if self.error.is_some() {
            return;
        }
        let attrs = remove_sorted_attr(&expr.attrs);
        if expr.attrs.len() != attrs.len() {
            expr.attrs = attrs;
            let result = match extract_arm_idents(&expr.arms) {
                Ok(entries) => check_path_order(entries),
                Err(err) => Err(err)
            };
            if let Err(err) = result {
                self.error = Some(err);
            }
        }
    }
}

fn extract_arm_idents(arms: &Vec<Arm>) -> syn::Result<Vec<PathEntry>> {
    let mut result = vec![];
    for arm in arms {
        match &arm.pat {
            Pat::Path(pat_path) => {
                result.push(PathEntry{name: path_to_string(&pat_path.path), path: &pat_path.path});
            }
            Pat::TupleStruct(pat_tuple) => {
                result.push(PathEntry{name: path_to_string(&pat_tuple.path), path: &pat_tuple.path});
            }
            Pat::Struct(pat_struct)=>{
                result.push(PathEntry { name: path_to_string(&pat_struct.path), path: &pat_struct.path });
            }
            _ => {
                return Err(syn::Error::new(arm.span(), "unsupported by #[sorted]"));
            }
        }
    }
    Ok(result)
}

fn path_to_string(path: &Path)-> String{
    path.segments.iter().
        map(|segment|segment.ident.to_string()).
        collect::<Vec<_>>().
        join("::")
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

