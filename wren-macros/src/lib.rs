#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
#![warn(unsafe_code)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::option_if_let_else)]

mod foreign_static_method;
mod generate_tests;
mod to_signature;

use foreign_static_method::internal_function_name;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use to_signature::{create_signature, ToSignatureInput};

#[proc_macro]
pub fn to_signature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as ToSignatureInput);
    create_signature(&data)
}

#[proc_macro]
pub fn call_signature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input with ToSignatureInput::parse_call_signature);
    create_signature(&data)
}

#[proc_macro]
pub fn generate_tests(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let output = generate_tests::generate_tests().unwrap_or_else(syn::Error::into_compile_error);
    output.into()
}

#[proc_macro]
pub fn foreign(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as syn::Ident);
    let output = internal_function_name(&input);
    quote!(#output).into()
}

#[proc_macro_attribute]
pub fn foreign_static_method(
    _args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as ItemFn);
    foreign_static_method::foreign_static_method(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
