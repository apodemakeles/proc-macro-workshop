use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Attribute, Data, DeriveInput, Expr, Field, Fields, GenericArgument, Lit, Meta, MetaNameValue, parse_macro_input, PathArguments, Type, TypePath};
use syn::punctuated::Iter;
use syn::spanned::Spanned;

fn only_type_path(f: &&Field) -> bool{
    match &f.ty {
        Type::Path(_) => true,
        _ => false
    }
}

fn convert<F>(data: &Data, f: F)-> TokenStream where F: Fn(Iter<Field>) -> TokenStream {
    if let Data::Struct(ref data) = *data {
        if let Fields::Named(ref fields) = data.fields {
            return f(fields.named.iter());
        }
    }
    unimplemented!()
}

fn build_function_macro(data: &Data) -> TokenStream {
    convert(data, |fields| {
        let items = fields.filter(only_type_path).map(|f| {
            let name = &f.ident;
            let ty = &f.ty;
            if is_type(ty, "Option") || is_type(ty, "Vec"){
                quote! {
                    #name: self.#name.clone()
                }
            } else {
                quote! {
                    #name: self.#name.clone().unwrap()
                }
            }
        });
        quote! {
            #(#items),*
        }
    })
}

fn build_check_macro(data: &Data) -> TokenStream{
    convert(data, |fields| {
        let items = fields.filter(only_type_path).
            filter(|f|{
                !is_type(&f.ty, "Option") && !is_type(&f.ty, "Vec")
            })
            .map(|f| {

                let name = &f.ident;
                quote! {
                if self.#name.is_none(){
                    return std::result::Result::Err(format!("{} missing", stringify!(#name)).into());
                }
            }
            });
        quote! {
            #(#items)*
        }
    })
}

fn setter_function_macro(data: &Data) -> TokenStream{
    convert(data, |fields|{
        let items = fields.filter(only_type_path).map(|f| {
            let name_ident = f.clone().ident.unwrap();
            let name = name_ident.to_string();
            let ty = &f.ty;
            if is_type(ty, "Option"){
                let param_ty = extract_generic_type(ty).unwrap();
                return quote! {
                    pub fn #name_ident(&mut self, #name_ident: #param_ty) -> &mut Self{
                        self.#name_ident = std::option::Option::Some(#name_ident);
                        self
                    }
                }
            }
            if is_type(ty, "Vec"){
                let builder_attr = extract_builder_attr(f);
                if builder_attr.is_none(){
                    return quote! {
                        pub fn #name_ident(&mut self, #name_ident: #ty) -> &mut Self{
                            self.#name_ident = #name_ident;
                            self
                        }
                    }
                }
                let builder_attr = builder_attr.unwrap();
                let each_name = extract_each_name(builder_attr);
                if let Err(ref err) = each_name{
                    return err.to_compile_error();
                }
                let each_name = each_name.unwrap();
                let each_name_ident = Ident::new(each_name.as_str(), f.span());
                let param_ty = extract_generic_type(ty).unwrap();
                let each_set_fn = quote! {
                    pub fn #each_name_ident(&mut self, #each_name_ident: #param_ty) -> &mut Self{
                        self.#name_ident.push(#each_name_ident);
                        self
                    }
                };
                let set_fn = if each_name.as_str() != name{
                    quote! {
                        pub fn #name_ident(&mut self, #name_ident: #ty) -> &mut Self{
                            self.#name_ident = #name_ident;
                            self
                        }
                    }
                }else{
                    quote! {}
                };
                return quote! {
                    #set_fn
                    #each_set_fn
                }
            }

            return quote! {
                pub fn #name_ident(&mut self, #name_ident: #ty) -> &mut Self{
                    self.#name_ident = std::option::Option::Some(#name_ident);
                    self
                }
            }

        });
        quote! {
            #(#items)*
        }
    })
}

fn builder_field_macro(data: &Data) -> TokenStream{
    convert(data, |fields|{
        let items = fields.filter(only_type_path).map(|f| {
            let name = &f.ident;
            let ty = &f.ty;
            if is_type(ty, "Option") || is_type(ty, "Vec"){
                quote! {
                    #name: #ty
                }
            }else{
                quote! {
                    #name: std::option::Option<#ty>
                }
            }
        });
        quote! {
            #(#items),*
        }
    })
}

fn extract_generic_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { qself: None, path }) = ty {
        if let Some(segment) = path.segments.last() {
            if let PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                if let Some(GenericArgument::Type(generic_type)) = generic_args.args.first() {
                    return Some(generic_type);
                }
            }
        }
    }
    None
}

fn is_type(ty: &Type, ty_name: &'static str) -> bool{
    if let Type::Path(type_path) = ty{
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == ty_name {
                if let PathArguments::AngleBracketed(generic_args) = &segment.arguments {
                    return generic_args.args.len() == 1;
                }
            }
        }
    }
    false
}

fn extract_builder_attr(f: &Field) -> Option<&Attribute>{
    f.attrs.iter().find(|attr|{
        attr.path().is_ident("builder")
    })
}

fn extract_each_name(builder_attr: &Attribute) -> Result<String, syn::Error> {
    if let Meta::List(meta_list) = &builder_attr.meta{
        let kv = meta_list.parse_args::<MetaNameValue>().unwrap();
        if !kv.path.is_ident("each"){
            return Err(syn::Error::new(meta_list.span(), "expected `builder(each = \"...\")`"));
        }
        if let Expr::Lit(lit) = kv.value{
            if let Lit::Str(str) = lit.lit{
                return Ok(str.value());
            }
        }
    }

    return Err(syn::Error::new(builder_attr.span(), "expected `builder(each = \"...\")`"));
}


#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let builder_name = format_ident!("{}Builder", name);

    let fields = builder_field_macro(&input.data);
    let builder_def = quote! {
        #[derive(Default)]
        pub struct #builder_name{
            #fields
        }
    };

    let checks = build_check_macro(&input.data);
    let assigns = build_function_macro(&input.data);
    let build_fn = quote! {
        pub fn build(&mut self) -> std::result::Result<#name, std::boxed::Box<dyn std::error::Error>>{
            #checks
            Ok(Command{
                #assigns
            })
        }
    };
    let functions = setter_function_macro(&input.data);
    let setter_fn = quote! {
        impl #builder_name{
            #functions
            #build_fn
        }
    };

    let builder_impl = quote! {
        impl #name{
            pub fn builder() -> #builder_name{
                Default::default()
            }
        }
    };

    let tokens = quote! {
        #builder_def
        #setter_fn
        #builder_impl
    };

    tokens.into()
}
