#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unknown_lints)]
#![allow(dead_code)]
#![allow(deref_nullptr)]
#![allow(improper_ctypes)]
#![allow(clippy::all)]
#![allow(clippy::pedantic, clippy::nursery, unsafe_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
