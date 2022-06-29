use std::ffi::CString;

use proc_macro2::Span;
use quote::quote;
use syn::{parenthesized, parse::Parse, parse_macro_input, punctuated::Punctuated, Token};

#[derive(Debug)]
struct ToSignatureInput {
    ident: syn::Ident,
    has_params: bool,
    param_count: usize,
}

type Fields = Punctuated<syn::Expr, Token![,]>;

impl Parse for ToSignatureInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        let lookahead = input.lookahead1();
        if lookahead.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let fields = Fields::parse_terminated(&content)?;
            Ok(ToSignatureInput {
                ident,
                has_params: true,
                param_count: fields.len(),
            })
        } else {
            Ok(ToSignatureInput {
                ident,
                has_params: false,
                param_count: 0,
            })
        }
    }
}

#[proc_macro]
pub fn to_signature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as ToSignatureInput);
    let mut ident = data.ident.to_string();

    if data.has_params {
        ident += "(";
        ident += &(0..data.param_count)
            .map(|_| "_")
            .collect::<Vec<&str>>()
            .join(",");
        ident += ")";
    }
    let ident = CString::new(ident).unwrap();

    let lit = syn::LitByteStr::new(ident.as_bytes_with_nul(), Span::call_site());
    let output = quote!(#lit);
    output.into()
}
