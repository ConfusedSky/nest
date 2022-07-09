use proc_macro2::{Span, TokenStream};
use std::path::PathBuf;

pub fn generate_tests() -> syn::Result<TokenStream> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("WE SHOULD HAVE ACCESS TO THE MANIFEST DIR");
    let dir = PathBuf::from(manifest_dir + "/scripts/test");
    let mut output = TokenStream::new();

    for file in dir.read_dir().expect("We can read the directory").flatten() {
        let file: PathBuf = file.file_name().into();
        let file = file
            .to_str()
            .expect("Failed to convert file path to string");
        let script = "scripts/test/".to_string() + file;
        let name = file.split('.').next().unwrap();

        let file_identifier = syn::Ident::new(name, Span::call_site());
        let fun = quote::quote!(
            #[test]
            fn #file_identifier() -> Result<(), Box<dyn std::error::Error>> {
                test_script(#script)
            }
        );

        output.extend(fun)
    }

    Ok(output)
}
