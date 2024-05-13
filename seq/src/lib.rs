use std::mem;
use std::process::id;
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Span, TokenStream, TokenTree};
use syn::{braced, LitInt, parse_macro_input, Token};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;

struct Seq{
    number: String,
    start: isize,
    end: isize,
    body: TokenStream,
}

enum StateStep{
    PrefixTilde,
    Number,
    SuffixTilde,
    Full
}

struct ReplaceContext {
    number: String,
    step: StateStep,
    name: String,
    span: Span,
    lit: bool,
}

impl ReplaceContext {

    fn from_number_ident(number_ident: &Ident, n: isize) -> ReplaceContext{
        ReplaceContext{
            number: number_ident.to_string(),
            step: StateStep::Number,
            name: n.to_string(),
            span: number_ident.span(),
            lit: true,
        }
    }

    fn from_prefix_idents(number: &str, ident: &Ident, _tilde: &Punct) -> ReplaceContext{
        ReplaceContext{
            number: number.to_string(),
            step: StateStep::PrefixTilde,
            name: ident.to_string(),
            span: ident.span(),
            lit: false
        }
    }

    fn push_number(&mut self, _number_ident: &Ident, n: isize){
        self.step = StateStep::Number;
        // self.span = self.span.join(number_ident.span()).unwrap();
        self.name.push_str(&n.to_string());
    }

    fn push_suffix_tilde(&mut self, suffix_tilde: &Punct){
        self.step = StateStep::SuffixTilde;
        // self.span = self.span.join(suffix_tilde.span()).unwrap();
        self.lit = false;
    }

    fn push_suffix_ident(&mut self, suffix_ident: &Ident){
        self.step = StateStep::Full;
        // self.span = self.span.join(suffix_ident.span()).unwrap();
        self.lit = false;
        self.name.push_str(&suffix_ident.to_string());
    }
}

impl TryFrom<ReplaceContext> for TokenTree{
    type Error = syn::Error;

    fn try_from(ctx: ReplaceContext) -> Result<Self, Self::Error> {
        return match ctx.step{
            StateStep::Number =>{
                if ctx.lit {
                    let mut lit = Literal::isize_unsuffixed(isize::from_str_radix(&ctx.name, 10).unwrap());
                    lit.set_span(ctx.span);
                    Ok(TokenTree::Literal(lit))
                }else{
                    Ok(TokenTree::Ident(Ident::new(&ctx.name, ctx.span)))
                }
            },
            StateStep::Full =>{
                Ok(TokenTree::Ident(Ident::new(&ctx.name, ctx.span)))
            },
            StateStep::PrefixTilde =>{
                Err(syn::Error::new(ctx.span, format!("must be followed with a(n) {}", ctx.number)))
            }
            StateStep::SuffixTilde=>{
                Err(syn::Error::new(ctx.span, "must be followed with identity"))
            }
        }
    }
}

impl Seq {
    fn replace_number(&self, tokens: TokenStream, n: isize) -> syn::Result<TokenStream> {
        let nodes = tokens.into_iter().collect::<Vec<_>>();
        let mut i = 0usize;
        let mut result: Vec<TokenTree> = vec![];
        while i < nodes.len() {
            let node = &nodes[i];
            match node {
                TokenTree::Punct(tilde) if tilde.to_string() == "~" => {
                    if result.len() == 0 {
                        return Err(syn::Error::new(node.span(), "before ~ must be a identity"));
                    }
                    result.remove(result.len() - 1);
                    let pre_node = &nodes[i - 1];
                    if let TokenTree::Ident(ref ident) = pre_node {
                        let ctx = ReplaceContext::from_prefix_idents(&self.number, ident, tilde);
                        i += 1;
                        let new_node = self.expand_number(&nodes, ctx, &mut i, n)?;
                        result.push(new_node);
                    } else {
                        return Err(syn::Error::new(node.span(), "before ~ must be a identity"));
                    }
                },
                TokenTree::Ident(number_ident)  if number_ident.to_string() == self.number => {
                    let ctx = ReplaceContext::from_number_ident(number_ident, n);
                    i += 1;
                    let new_node = self.expand_number(&nodes, ctx, &mut i, n)?;
                    result.push(new_node);
                },
                TokenTree::Group(group) => {
                    let mut new_group = Group::new(group.delimiter(), self.replace_number(group.stream(), n)?);
                    new_group.set_span(group.span());
                    result.push(TokenTree::Group(new_group));
                    i += 1;
                },
                _ => {
                    result.push(node.clone());
                    i += 1;
                }
            }
        }

        Ok(convert_token_trees_to_stream(result))
    }

    fn expand_number(&self, nodes: &Vec<TokenTree>, mut ctx: ReplaceContext, idx: &mut usize, n: isize) -> syn::Result<TokenTree> {
        while *idx < nodes.len() {
            let node = &nodes[*idx];
            match ctx.step {
                StateStep::PrefixTilde => {
                    if let TokenTree::Ident(number_ident) = node {
                        if number_ident.to_string() == self.number {
                            ctx.push_number(number_ident, n);
                            *idx += 1;
                            continue;
                        }
                    }
                    return Err(syn::Error::new(node.span(), format!("must be {}", self.number)));
                },
                StateStep::Number => {
                    if let TokenTree::Punct(punct) = node {
                        if punct.to_string() == "~" {
                            ctx.push_suffix_tilde(punct);
                            *idx += 1;
                            continue;
                        }
                    }
                    return ctx.try_into();
                },
                StateStep::SuffixTilde => {
                    if let TokenTree::Ident(suffix_ident) = node {
                        ctx.push_suffix_ident(suffix_ident);
                        *idx += 1;
                        continue;
                    }
                    return Err(syn::Error::new(node.span(), "must be a identity"));
                },
                StateStep::Full => return ctx.try_into()
            }
        }

        ctx.try_into()
    }

    fn expand_body(&self, tokens: TokenStream) -> syn::Result<(TokenStream, bool)>{
        let buffer = syn::buffer::TokenBuffer::new2(tokens);
        let mut cursor = buffer.begin();
        let mut found = false;
        let mut result = Tokens::default();

        while !cursor.eof() {
            if let Some((punct, next_cursor)) = cursor.punct() {
                if punct.as_char() == '#' {
                    if let Some((group_begin, _, group_end)) = next_cursor.group(Delimiter::Parenthesis){
                        if let Some((punct, next_cursor)) = group_end.punct(){
                            if punct.as_char() == '*'{
                                found = true;
                                cursor = next_cursor;
                                result.extend(self.expand_content(&group_begin.token_stream())?);
                                // eprintln!("{:#?}", group_begin.token_stream());
                                continue;
                            }
                        }
                    }
                }
            }

            if let Some((node, next_cursor)) = cursor.token_tree(){
                match node{
                    TokenTree::Group(group)=>{
                        let (group_stream, found2) = self.expand_body(group.stream())?;
                        let mut new_group = Group::new(group.delimiter(), group_stream);
                        new_group.set_span(group.span());
                        result.push(TokenTree::Group(new_group));
                        found = found || found2;
                    },
                    _ => result.push(node),
                }
                cursor = next_cursor;
            }else{
                unreachable!()
            }
        }

        Ok((result.into(), found))
    }


    fn expand_content(&self, content: &TokenStream) -> syn::Result<TokenStream>{
        let mut result = TokenStream::new();
        for i in self.start..self.end {
            let body = self.replace_number(content.clone(), i)?;
            result.extend(body);
        }

        Ok(result)
    }
}

#[derive(Default)]
struct Tokens {
    inner: TokenStream,
    nodes: Vec<TokenTree>
}

impl Tokens{
    fn push(&mut self, node: TokenTree){
        self.nodes.push(node);
    }

    fn flush(&mut self){
        if !self.nodes.is_empty(){
            let nodes = mem::take(&mut self.nodes);
            self.inner.extend(convert_token_trees_to_stream(nodes));
        }
    }

    fn extend(&mut self, token_stream: TokenStream){
        self.flush();
        self.inner.extend(token_stream);
    }
}

impl From<Tokens> for TokenStream{
    fn from(mut tokens: Tokens) -> Self {
        tokens.flush();
        tokens.inner
    }
}

fn convert_token_trees_to_stream(token_trees: Vec<TokenTree>) -> TokenStream {
    let mut token_stream = TokenStream::new();

    for token_tree in token_trees {
        token_stream.extend(Some::<TokenStream>(token_tree.into()));
    }

    token_stream
}


impl Parse for Seq{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let number = input.parse::<Ident>()?.to_string();
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
    match input.expand_body(input.body.clone()) {
        Ok((tokens, true)) => tokens,
        Ok((_, false))=>{
            input.expand_content(&input.body).unwrap_or_else(|err| err.into_compile_error())
        },
        Err(err) => {
            err.into_compile_error()
        }
    }.into()
}