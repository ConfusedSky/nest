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
