#!allow(unsafe_code);

mod scheduler;

use std::collections::HashMap;
use std::ffi::CString;

lazy_static! {
    // TODO: Refactor to make this not require modules to stay in memory indefinitely
    static ref MODULES: HashMap<&'static str, CString> = {
        let mut m = HashMap::new();
        let scheduler = include_str!("scheduler.wren");
        m.insert("scheduler", CString::new(scheduler).unwrap());

        m
    };
}

pub fn get_module<S>(name: S) -> Option<&'static CString>
where
    S: AsRef<str>,
{
    MODULES.get(name.as_ref())
}
