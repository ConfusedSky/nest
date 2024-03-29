use proc_macro2::{Span, TokenStream};
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

#[allow(clippy::unnecessary_wraps)]
pub fn generate_mod(path: &Path, mod_name: Option<&str>) -> syn::Result<TokenStream> {
    let mut output = TokenStream::new();

    for file in path
        .read_dir()
        .expect("We can read the directory")
        .flatten()
    {
        let file_type = file.file_type().expect("Could not get file type");
        let file_path = &path.join(file.path());
        let file_name: PathBuf = file.file_name().into();
        let file_name = file_name
            .to_str()
            .expect("Failed to convert file path to string");

        if file_type.is_dir() {
            output.extend(generate_mod(file_path, Some(file_name)));
            continue;
        } else if file_type.is_symlink() {
            continue;
        }

        let file_contents =
            read_to_string(file_path).unwrap_or_else(|_| panic!("Could not read {file_path:?}"));
        if file_contents.contains("// Skip") {
            continue;
        }

        let split = file_name.split('.').collect::<Vec<_>>();
        if let [name, extension] = &split[..] {
            if *extension != "wren" {
                continue;
            }

            let file_identifier = syn::Ident::new(name, Span::call_site());
            let script_path = file_path;
            let script = script_path.to_string_lossy();
            let fun = quote::quote!(
                #[test]
                fn #file_identifier() -> Result<(), Box<dyn std::error::Error>> {
                    crate::test_script(#script)
                }
            );

            output.extend(fun);
        }
    }

    if let Some(name) = mod_name {
        let name = syn::Ident::new(name, Span::call_site());
        output = quote::quote!(
            mod #name {
                #output
            }
        );
    }

    Ok(output)
}

#[allow(clippy::unnecessary_wraps)]
pub fn generate_tests() -> syn::Result<TokenStream> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("WE SHOULD HAVE ACCESS TO THE MANIFEST DIR");
    let dir = PathBuf::from(manifest_dir).join("test");
    generate_mod(&dir, None)
}
