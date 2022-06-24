#![allow(unsafe_code)]

use crate::wren::VMPtr;

use super::{source_file, Class, Module};
use std::io::{stdout, Write};

pub fn init_module() -> Module {
    let mut stdout_class = Class::new();
    stdout_class.static_methods.insert("flush()", flush);

    let mut timer_module = Module::new(source_file!("io.wren"));
    timer_module.classes.insert("Stdout", stdout_class);

    timer_module
}

fn flush(vm: VMPtr) {
    stdout().flush().map_or_else(
        |_| {
            vm.abort_fiber("Stdout failed to flush");
        },
        |_| {
            vm.set_return_value(&());
        },
    );
}
