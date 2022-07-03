mod generate_tests;
mod to_signature;
use syn::parse_macro_input;
use to_signature::{create_signature, ToSignatureInput};

#[proc_macro]
pub fn to_signature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input as ToSignatureInput);
    create_signature(data)
}

#[proc_macro]
pub fn call_signature(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let data = parse_macro_input!(input with ToSignatureInput::parse_call_signature);
    create_signature(data)
}

#[proc_macro]
pub fn generate_tests(_input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let output = generate_tests::generate_tests().unwrap_or_else(|e| e.into_compile_error());
    output.into()
}
