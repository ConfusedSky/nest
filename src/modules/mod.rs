#!allow(unsafe_code);

macro_rules! cstr {
    ($s:expr) => {
        (concat!($s, "\0") as *const str as *const [::std::os::raw::c_char]).cast::<i8>()
    };
}

macro_rules! source_file {
    ($file:expr) => {{
        use std::ffi::CString;
        let source = cstr!(include_str!($file));
        unsafe { CString::from_raw(source as *mut _) }
    }};
}

pub mod io;
pub mod os;
pub mod scheduler;
pub mod timer;

use crate::wren;
use std::collections::HashMap;
use std::ffi::CString;

pub struct Class {
    pub methods: HashMap<&'static str, wren::ForeignMethod>,
    pub static_methods: HashMap<&'static str, wren::ForeignMethod>,
}

impl Class {
    fn new() -> Self {
        Self {
            methods: HashMap::new(),
            static_methods: HashMap::new(),
        }
    }
}

pub struct Module {
    pub source: CString,
    pub classes: HashMap<&'static str, Class>,
}

impl Module {
    fn new(source: CString) -> Self {
        Self {
            source,
            classes: HashMap::new(),
        }
    }
}

fn modules_init() -> HashMap<&'static str, Module> {
    let mut m = HashMap::new();
    m.insert("scheduler", scheduler::init_module());
    m.insert("timer", timer::init_module());
    m.insert("os", os::init_module());
    m.insert("io", io::init_module());

    m
}

lazy_static! {
    // TODO: Refactor to make this not require modules to stay in memory indefinitely
    static ref MODULES: HashMap<&'static str, Module> = {
        modules_init()
    };
}

pub fn get_module<S>(name: S) -> Option<&'static Module>
where
    S: AsRef<str>,
{
    MODULES.get(name.as_ref())
}
