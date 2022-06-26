#!allow(unsafe_code);

pub mod io;
pub mod os;
pub mod scheduler;
pub mod timer;

use crate::ForeignMethod;
use std::collections::HashMap;
use std::ffi::CStr;

mod macros {
    #[macro_export]
    macro_rules! source_file {
        ($file:expr) => {{
            use crate::wren;
            use std::ffi::CStr;
            let source = wren::cstr!(include_str!($file));
            unsafe { CStr::from_ptr(source) }
        }};
    }
    pub use source_file;
}
use macros::source_file;

pub struct Class<'wren> {
    pub methods: HashMap<&'static str, ForeignMethod<'wren>>,
    pub static_methods: HashMap<&'static str, ForeignMethod<'wren>>,
}

impl<'wren> Class<'wren> {
    fn new() -> Self {
        Self {
            methods: HashMap::new(),
            static_methods: HashMap::new(),
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
}

impl<'wren> Modules<'wren> {
    pub fn new() -> Self {
        let mut m = HashMap::new();
        m.insert("scheduler", scheduler::init_module());
        m.insert("timer", timer::init_module());
        m.insert("os", os::init_module());
        m.insert("io", io::init_module());

        Modules { hash_map: m }
    }

    pub fn get_module<S>(&self, name: S) -> Option<&Module<'wren>>
    where
        S: AsRef<str>,
    {
        self.hash_map.get(name.as_ref())
    }
}
