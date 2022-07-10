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
//!   Otherwise it should be have the same type as the context passed in [ ] [ ]
//!   Make sure to check that it is a foreign context and error if it isn't [ ] [ ]
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
//! - Use `Result<T: SetValue, String>` as a shorthand to be able to abort the calling fiber [ ] [ ]
//!
//! - Add a trait that modules can implement that allows them to be accessed by the VM
//!   so we can support instance methods for modules [ ] [ ]
//!

use proc_macro2::{Ident, Span, TokenStream};
use proc_macro_crate::{crate_name, FoundCrate};
use quote::{quote, quote_spanned};
use syn::{
    punctuated::Punctuated, spanned::Spanned, FnArg, ItemFn, LitInt, Pat, PatType, Token, Type,
    TypeReference,
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

    fn names(&self) -> impl Iterator<Item = &Box<Pat>> {
        self.args.iter().map(|arg| &arg.pat)
    }

    fn get_slot(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.args.iter().enumerate().map(|(i, pattern)| {
            let arg_name = &pattern.pat;
            let arg_type = &pattern.ty;
            // Start at 1 instead of 0 to make sure that we read the arguments
            // rather than the Class
            let i = LitInt::new(&(i + 1).to_string(), Span::call_site());

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

pub fn foreign_static_method(input: &ItemFn) -> syn::Result<TokenStream> {
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
    let arg_get_slot = args.get_slot();
    let context_ref = args.context_ref();
    let generics = if args.context_type.is_some() {
        quote!()
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
    let type_assertion = if let Some((span, _)) = &args.context_type {
        quote_spanned!(*span =>
            fn __assert_is_foreign<_V>(_: &mut #wren_crate::Context<_V, #wren_crate::context::Foreign>) {}
            __assert_is_foreign(&mut context);
        )
    } else {
        quote!()
    };

    let internal_function_name = internal_function_name(name);

    Ok(quote!(
        #input

        fn #internal_function_name #generics(
            mut context: #context_type
        ) {
            use #wren_crate::{GetValue, SetValue};
            #type_assertion

            unsafe {
                #(#arg_get_slot)*
                #name(#context_ref #(#arg_names),*)
                    .set_slot(&mut context, 0);
            }
        }
    ))
}

pub fn internal_function_name(name: &Ident) -> Ident {
    quote::format_ident!("__wren_internal_{}", name)
}
