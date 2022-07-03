use proc_macro2::{Span, TokenStream};
use std::path::PathBuf;

pub fn generate_tests() -> syn::Result<TokenStream> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("WE SHOULD HAVE ACCESS TO THE MANIFEST DIR");
    let dir = PathBuf::from(manifest_dir + "/scripts/test");
    let mut output = TokenStream::new();

    for file in dir.read_dir().expect("We can read the directory").flatten() {
        let file = file.file_name();
        let file = file.to_str().ok_or_else(|| {
            syn::Error::new(Span::call_site(), "Failed to convert filename to string")
        })?;
        let file = file.split('.').next().ok_or_else(|| {
            syn::Error::new(
                Span::call_site(),
                "failed to remove the extension from script name",
            )
        })?;
        let script = "test/".to_string() + file;

        let file_identifier = syn::Ident::new(&file, Span::call_site());
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
