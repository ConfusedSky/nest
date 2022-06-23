#![allow(unsafe_code)]

use crate::wren::{Null, VMPtr};

use super::{Class, Module};
use std::{
    ffi::CString,
    io::{stdout, Write},
};

pub fn init_module() -> Module {
    let timer_source = include_str!("io.wren");

    let mut stdout_class = Class::new();
    stdout_class.static_methods.insert("flush()", flush);

    let mut timer_module = Module::new(CString::new(timer_source).unwrap());
    timer_module.classes.insert("Stdout", stdout_class);

    timer_module
}

fn flush(vm: VMPtr) {
    stdout().flush().map_or_else(
        |_| {
            vm.abort_fiber("Stdout failed to flush");
        },
        |_| {
            vm.set_return_value(&Null);
        },
    );
}
