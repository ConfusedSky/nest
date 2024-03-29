//! # Goals
//! Note here the first checkbox here is implementation and the second one is tested
//!
//! - Create a wrapper around a function that makes `get_slot` calls to get
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
//!     calls `get_slot` or `get_slot_unchecked` and the wren end user has to be more
//!     careful respectively [ ] [ ]
//!
//! - Optionally support results for the `try_get` methods [ ] [ ]
//!
//! - Allow the user to leave off the context in their arguments [x] [x]
//!   If they leave off the context from their arguments then the
//!     context user data should be a generic [ ] [ ]
//!   Otherwise it should be have the same type as the context passed in [x] [x]
//!   Make sure to check that it is a foreign context and error if it isn't [x] [x]
//!
//! - Make sure to respect visibility [ ] [ ]
//!
//! - Have good error messages
//!   Make sure the context is always the first item in the argument list [x] [x]
//!   Make sure to check to make sure vm argument is a reference [x] [x]
//!   Have errors saying which argument has an invalid type [ ] [ ]
//!
//! - Generate better errors for bad return values
//!   Make sure the error appears at the return value [ ] [ ]
//!   Create a custom error message for a bad return value type [ ] [ ]
//!
//! - Use `Result<T: SetValue, String>` as a shorthand to be able to abort the calling fiber [x] [x]
//!
//! - Add a trait that modules can implement that allows them to be accessed by the VM
//!   so we can support instance methods for modules [ ] [ ]
//!   Also that way we don't have to expose the context to foreign functions really at all
//!
//! - Allow non context arguments to be passed as references [ ] [ ]

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, quote_spanned};
use syn::{
    punctuated::Punctuated, spanned::Spanned, FnArg, ItemFn, LitInt, Pat, PatType, PathArguments,
    Token, Type, TypeReference,
};

struct Arguments {
    args: Vec<PatType>,
    context_type: Option<(Span, TypeReference)>,
}

impl Arguments {
    fn context_inner_type(&self) -> Option<&Type> {
        self.context_type.as_ref().map(|(_, ty)| &*ty.elem)
    }

    fn context_ref(&self) -> TokenStream {
        if let Some((span, ty)) = &self.context_type {
            let ref_type = if ty.mutability.is_some() {
                quote!(&mut)
            } else {
                quote!(&)
            };

            quote_spanned!(*span=> #ref_type context,)
        } else {
            TokenStream::new()
        }
    }

    fn names(&self) -> impl Iterator<Item = Box<Pat>> + '_ {
        self.args.iter().map(|arg| {
            let mut pat = arg.pat.clone();

            // We just want the names here so we remove mutability specifiers
            if let Pat::Ident(ref mut i) = *pat {
                i.mutability = None;
            }

            pat
        })
    }

    fn get_slot(&self, is_static: bool) -> impl Iterator<Item = TokenStream> + '_ {
        self.args.iter().enumerate().map(move |(i, pattern)| {
            let mut arg_name = pattern.pat.clone();
            let mut arg_type = *pattern.ty.clone();

            // We just want the names here so we remove mutability specifiers
            if let Pat::Ident(ref mut pat) = *arg_name {
                pat.mutability = None;
            }

            // Start at 1 instead of 0 to make sure that we read the arguments
            // rather than the Class for static methods
            // otherwise start at 0
            let i = LitInt::new(
                &(i + if is_static { 1 } else { 0 }).to_string(),
                Span::call_site(),
            );

            // Change type parameters to use turbofish when calling
            // get_slot because it is in the expression position
            if let Type::Path(ref pat) = &arg_type {
                let ident = &pat.path.segments[0].ident;
                let arguments = &pat.path.segments[0].arguments;

                if *arguments != PathArguments::None {
                    arg_type = syn::parse_quote!(#ident::#arguments);
                }
            }

            quote_spanned!(
                pattern.span() =>
                    let #arg_name =
                        #arg_type::get_slot(
                            &mut context,
                            #i
                        );
            )
        })
    }
}

fn type_is_context(path: &Type) -> bool {
    if let Type::Path(ref pat) = path {
        if pat.path.segments[0].ident == "Context" {
            return true;
        }
    }

    false
}

impl TryFrom<&Punctuated<FnArg, Token![,]>> for Arguments {
    type Error = syn::Error;
    fn try_from(args: &Punctuated<FnArg, Token![,]>) -> syn::Result<Self> {
        let mut context_type = None;
        let args = args
            .into_iter()
            .enumerate()
            .filter_map(|(i, argument)| {
                if let syn::FnArg::Typed(pattern) = argument {
                    if let Type::Reference(ref ty) = &*pattern.ty {
                        if type_is_context(&*ty.elem) {
                            if i != 0 {
                                return Some(Err(syn::Error::new(
                                    argument.span(),
                                    "Context argument must be first argument",
                                )));
                            }

                            context_type = Some((argument.span(), ty.clone()));
                            return None;
                        }
                    } else if type_is_context(&*pattern.ty) {
                        return Some(Err(syn::Error::new(
                            argument.span(),
                            "Context argument must be a reference",
                        )));
                    }

                    Some(Ok(pattern.clone()))
                } else {
                    Some(Err(syn::Error::new(
                        argument.span(),
                        "This macro doesn't support instance methods",
                    )))
                }
            })
            .collect::<syn::Result<_>>()?;

        Ok(Self { args, context_type })
    }
}

pub fn foreign_method(input: &ItemFn, is_static: bool) -> syn::Result<TokenStream> {
    let name = &input.sig.ident;

    let wren_crate = {
        let crate_name = crate_name("wren").expect("wren must be present for this macro");
        match crate_name {
            FoundCrate::Itself => Ident::new("crate", Span::call_site()),
            FoundCrate::Name(name) => Ident::new(&name, Span::call_site()),
        }
    };

    let args = Arguments::try_from(&input.sig.inputs)?;
    let arg_names = args.names();
    let arg_get_slot = args.get_slot(is_static);
    let context_ref = args.context_ref();
    let generics = if args.context_type.is_some() {
        let (input_generics, _, _) = input.sig.generics.split_for_impl();
        quote!(#input_generics)
    } else {
        quote!(<'wren, V: #wren_crate::VmUserData<'wren, V>>)
    };
    let context_type = if let Some(inner_type) = &args.context_inner_type() {
        quote!(#inner_type)
    } else {
        quote!(
            #wren_crate::Context<
                'wren,
                V,
                #wren_crate::context::Foreign
            >
        )
    };
    // TODO: Figure out how to get the span for this correct
    // This makes sure the errors from to_output are spanned to the context arg if it
    // exists, and allows type assertions without having a specific assertion
    let to_output = {
        let span = if let Some((span, _)) = &args.context_type {
            *span
        } else {
            Span::call_site()
        };

        quote_spanned!(span =>
            .to_output(&mut context);
        )
    };
    let internal_function_name = internal_function_name(name);

    Ok(quote!(
        #input

        fn #internal_function_name #generics(
            mut context: #context_type
        ) {
            use #wren_crate::{GetValue, SetValue, context::ForeignCallOutput};

            unsafe {
                #(#arg_get_slot)*
                let output = &#name(#context_ref #(#arg_names),*)#to_output

                if let Some(output) = output {
                    output.set_slot(&mut context, 0);
                }
            }
        }
    ))
}

pub fn internal_function_name(name: &Ident) -> Ident {
    quote::format_ident!("foreign_{}", name)
}
