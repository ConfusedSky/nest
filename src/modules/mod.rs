#!allow(unsafe_code);

pub mod bigint;
pub mod io;
pub mod os;
pub mod scheduler;
pub mod timer;

use crate::ForeignMethod;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::fs;
use wren::ForeignClassMethods;

mod macros {
    #[macro_export]
    macro_rules! source_file {
        ($file:expr) => {{
            use wren;
            wren::cstr!(include_str!($file))
        }};
    }
    pub use source_file;
}
use macros::source_file;

pub struct Class<'wren> {
    pub methods: HashMap<&'static str, ForeignMethod<'wren>>,
    pub static_methods: HashMap<&'static str, ForeignMethod<'wren>>,
    pub foreign_class_methods: Option<ForeignClassMethods>,
}

impl<'wren> Class<'wren> {
    fn new() -> Self {
        Self {
            methods: HashMap::new(),
            static_methods: HashMap::new(),
            foreign_class_methods: None,
        }
    }
}

pub struct Module<'wren> {
    pub source: &'wren CStr,
    pub classes: HashMap<&'static str, Class<'wren>>,
}

impl<'wren> Module<'wren> {
    fn new(source: &'wren CStr) -> Self {
        Self {
            source,
            classes: HashMap::new(),
        }
    }
}

pub struct Modules<'wren> {
    hash_map: HashMap<&'static str, Module<'wren>>,
    // This is the last CString that has been loaded
    // we keep a reference to it so wren can compile it
    last_loaded: Option<CString>,
}

impl<'wren> Modules<'wren> {
    pub fn new() -> Self {
        let mut m = HashMap::new();
        m.insert("scheduler", scheduler::init_module());
        m.insert("timer", timer::init_module());
        m.insert("os", os::init_module());
        m.insert("io", io::init_module());
        m.insert("bigint", bigint::init_module());

        Modules {
            hash_map: m,
            last_loaded: None,
        }
    }

    pub fn get_module<S>(&self, name: S) -> Option<&Module<'wren>>
    where
        S: AsRef<str>,
    {
        self.hash_map.get(name.as_ref())
    }

    pub fn load_module(&mut self, name: &str) -> Option<&CStr> {
        self.get_module(name)
            .map(|module| &module.source)
            .copied()
            .or_else(|| {
                let module = fs::read_to_string(name).ok()?;
                self.last_loaded = CString::new(module).ok();
                self.last_loaded.as_deref()
            })
    }
}
