//! # Goals
//! Note here the first checkbox here is implementation and the second one is tested
//!
//! - Create a wrapper around a function that makes get_slot calls to get
//!     the returned values [x] [x]
//!
//! - Make sure all foreign methods for a module are implemented at compile time
//!     IE: Have it be a compiler error if there are any foreign methods that haven't
//!     been implemented [ ] [ ]
//!
//! - Have the ability to optionally generate stub implementations that do some
//!     typechecking on the wren side for the public api of a class.
//!     Since we can't really do that on the rust side. [ ] [ ]
//!
//! - Have two modes strict and dynamic where depending on the type the function
//!     calls get_slot or get_slot_unchecked and the wren end user has to be more
//!     careful respectively [ ] [ ]
//!
//! - Optionally support results for the try_get methods [ ] [ ]
//!
//! - Allow the user to leave off the context in their arguments [ ] [ ]
//!   If they leave off the context from their arguments then the
//!     context user data should be a generic [ ] [ ]
//!   Otherwise it should be have the same type as the context passed in [ ] [ ]
//!   Make sure to check that it is a foreign context and error if it isn't [ ] [ ]
//!
//! - Generate better errors for bad return values
//!   Make sure the error appears at the return value [ ] [ ]
//!   Create a custom error message for a bad return value type [ ] [ ]
//!
//! - Make sure to respect visibility [ ] [ ]
//!
//! - Have good error messages
//!   Make sure the context is always the first item in the argument list [ ] [ ]
//!   Have errors saying which argument has an invalid type [ ] [ ]

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, quote_spanned};
use syn::{spanned::Spanned, ItemFn, LitInt};

pub fn foreign_static_method(mut input: ItemFn) -> syn::Result<TokenStream> {
    let name = input.sig.ident;
    let original_function_name = quote::format_ident!("__wren_internal_{}", name);
    input.sig.ident = original_function_name.clone();

    let wren_crate = {
        let crate_name = crate_name("wren").expect("wren must be present for this macro");
        match crate_name {
            FoundCrate::Itself => Ident::new("crate", Span::call_site()),
            FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        }
    };

    let args = input.sig.inputs.clone();
    let args = args
        .into_iter()
        .enumerate()
        .map(|(i, argument)| {
            if let syn::FnArg::Typed(pattern) = argument {
                let arg_name = pattern.pat.clone();
                let arg_type = &pattern.ty;
                // Start at 1 instead of 0 to make sure that we read the arguments
                // rather than the Class
                let i = LitInt::new(&(i + 1).to_string(), Span::call_site());

                let get_slot = quote_spanned!(
                    pattern.span() =>
                        let #arg_name =
                            #arg_type::get_slot(
                                &mut context,
                                #i
                            );
                );

                Ok((arg_name, get_slot))
            } else {
                Err(syn::Error::new(
                    argument.span(),
                    "This macro doesn't support instance methods",
                ))
            }
        })
        .collect::<syn::Result<Vec<_>>>()?;
    let arg_names = args.iter().map(|x| &x.0);
    let arg_get_slot = args.iter().map(|x| &x.1);

    Ok(quote!(
        #input

        unsafe fn #name<'wren, V: #wren_crate::VmUserData<'wren, V>>(
            mut context: #wren_crate::Context<
                'wren,
                V,
                #wren_crate::context::Foreign
            >
        ) {
            use #wren_crate::{GetValue, SetValue};
            #(#arg_get_slot)*
            #original_function_name(#(#arg_names),*)
                .set_slot(&mut context, 0);
        }
    ))
}
