use proc_macro2::{Span, TokenStream};
use std::path::{Path, PathBuf};

#[allow(clippy::unnecessary_wraps)]
pub fn generate_mod(path: &Path, mod_name: Option<&str>) -> syn::Result<TokenStream> {
    let mut output = TokenStream::new();

    for file in path
        .read_dir()
        .expect("We can read the directory")
        .flatten()
    {
        let file_type = file.file_type().expect("Could not get file type");
        let file_path = file.path();
        if file_type.is_dir() {
            output.extend(generate_mod(&path.join(file_path), None));
            continue;
        } else if file_type.is_symlink() {
            continue;
        }

        let file: PathBuf = file.file_name().into();
        let file = file
            .to_str()
            .expect("Failed to convert file path to string");
        let split = file.split('.').collect::<Vec<_>>();
        if let [name, extension] = &split[..] {
            if *extension != "wren" {
                continue;
            }

            let file_identifier = syn::Ident::new(name, Span::call_site());
            let script_path = &path.join(file_path);
            let script = script_path.to_string_lossy();
            let fun = quote::quote!(
                #[test]
                fn #file_identifier() -> Result<(), Box<dyn std::error::Error>> {
                    test_script(#script)
                }
            );

            output.extend(fun);
        }
    }

    if let Some(name) = mod_name {
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
