//! # GOALS
//! - Create a wrapper around a function that makes get_slot calls to get
//!   the returned values [ ]
//!
//! - Make sure all foreign methods for a module are implemented at compile time
//!   IE: Have it be a compiler error if there are any foreign methods that haven't
//!   been implemented [ ]
//!
//! - Have the ability to optionally generate stub implementations that do some
//!   typechecking on the wren side for the public api of a class.
//!   Since we can't really do that on the rust side. [ ]
//!
//! - Have two modes strict and dynamic where depending on the type the function
//!   calls get_slot or get_slot_unchecked and the wren end user has to be more
//!   careful respectively [ ]
//!
//! - Optionally support results for the try_get methods [ ]
//!
//! - Allow the user to leave off the context in their arguments [ ]
//!
//! - Have good error messages
//!   Make sure the context is always the first item in the argument list [ ]
//!   Have errors saying which argument has an invalid type [ ]

use proc_macro2::TokenStream;
use syn::ItemFn;

pub fn foreign_static_method(input: ItemFn) -> syn::Result<TokenStream> {
    Ok(quote::quote!(#input))
}
