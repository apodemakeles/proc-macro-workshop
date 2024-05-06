use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, ExprLit, Field, Fields, GenericArgument, GenericParam, Generics, Lit, Meta, MetaNameValue, parse_macro_input, parse_str, PathArguments, Type, TypePath, WhereClause, WherePredicate};
use syn::spanned::Spanned;

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_ident = input.ident;
    let fields = match &input.data {
        Data::Struct(data_struct) => &data_struct.fields,
        _ => unreachable!()
    };
    let fields_debug = field_debug_macro(fields);
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut new_where_clause = if let Some(where_clause) = where_clause{
        where_clause.clone()
    }else{
        WhereClause{
            where_token:Default::default(),
            predicates: Default::default(),
        }
    };
    let mut type_param_to_bound_set: HashSet<String> = HashSet::new();
    match get_type_param_to_bound_from_attr(&input.attrs){
        Err(err) => return err.into_compile_error().into(),
        Ok(Some(bounds)) => {
            // disable all inference of bounds
            bounds.into_iter().for_each(|bound| { type_param_to_bound_set.insert(bound); });
        }
        _=>{
            let type_param_set = get_all_generic_type_param_name_set(&input.generics);
            for field in fields {
                if let Some(expr) = get_name_of_type_or_associate_type_to_bound(&field.ty, &type_param_set){
                    type_param_to_bound_set.insert(format!("{}: std::fmt::Debug", expr));
                }
            }
        }
    }

    for string in type_param_to_bound_set {
        let debug_bound: WherePredicate  = parse_str(&string).expect("Failed to parse where predicate");
        new_where_clause.predicates.push(debug_bound);
    }

    let tokens = quote! {
        impl #impl_generics std::fmt::Debug for #struct_ident #ty_generics #new_where_clause{
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result{
                fmt.debug_struct(stringify!(#struct_ident)).#fields_debug.finish()
            }
        }
    };

    tokens.into()
}

fn field_debug_macro(fields: &Fields) -> TokenStream{
    let items = fields.iter().map(|f| {
        let field_ident = &f.ident;
        let argument = if let Some(format) = extract_debug_format(f){
            quote! { format_args!(#format, &self.#field_ident) }
        }else{
            quote! { &self.#field_ident }
        };
        quote! {
            field(stringify!(#field_ident), &#argument)
        }
    });

    quote! {
        #(#items).*
    }
}

fn extract_debug_format(field: &Field)-> Option<String> {
    field.attrs.iter().find_map(|attr| {
        if let Meta::NameValue(name_value) = &attr.meta {
            if name_value.path.is_ident("debug") {
                if let Expr::Lit(lit) = &name_value.value {
                    if let Lit::Str(str) = &lit.lit {
                        return Some(str.value());
                    }
                }
            }
        }
        None
    })
}

fn get_all_generic_type_param_name_set(generics: &Generics) -> HashSet<String> {
    generics.params.iter().filter_map(|param| {
        match param {
            GenericParam::Type(type_param) => Some(type_param.ident.to_string()),
            _ => None
        }
    }).collect()
}

fn get_name_of_type_or_associate_type_to_bound(ty: &Type, type_param_set: &HashSet<String>) -> Option<String>{
    if let Type::Path(TypePath { qself: None, path }) = ty {
        // for Foo<Bar<...<T>>>
        if let Some(segment) = path.segments.last() {
            let type_name = segment.ident.to_string();
            if type_param_set.contains(type_name.as_str()){
                return Some(type_name)
            }
            if type_name == "PhantomData"{
                return None;
            }
            if let PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                if let Some(GenericArgument::Type(generic_type)) = generic_args.args.first() {
                    return get_name_of_type_or_associate_type_to_bound(generic_type, type_param_set);
                }
            }
        }
        if path.segments.len() == 2 && type_param_set.contains(path.segments[0].ident.to_string().as_str()){
            return Some(path.segments.iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"))
        }
    }
    None
}

fn get_type_param_to_bound_from_attr(attrs: &[Attribute]) -> syn::Result<Option<Vec<String>>>{
    for attr in attrs {
        if let Meta::List(meta_list) = &attr.meta {
            let path = &meta_list.path;
            if path.is_ident("debug") {
                if let Ok(kv) = meta_list.parse_args::<MetaNameValue>() {
                    if kv.path.is_ident("bound") {
                        if let Expr::Lit( ExprLit { lit: Lit::Str(lit), ..}) = kv.value {
                            return Ok(Some(lit.value().split(",").map(|s|s.to_owned()).collect()))
                        }
                    }
                }
                return Err(syn::Error::new(meta_list.span(), "expected `debug(bound = \"...\")`"));
            }
        }
    }

    Ok(None)
}
