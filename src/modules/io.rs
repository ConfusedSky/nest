use crate::Context;

use super::{source_file, Class, Module};
use std::io::{stdout, Write};

pub fn init_module<'wren>() -> Module<'wren> {
    let mut stdout_class = Class::new();
    stdout_class.static_methods.insert("flush()", flush);

    let mut timer_module = Module::new(source_file!("io.wren"));
    timer_module.classes.insert("Stdout", stdout_class);

    timer_module
}

fn flush(mut vm: Context) {
    match stdout().flush() {
        Ok(_) => vm.set_return_value(&()),
        Err(_) => vm.abort_fiber("Stdout failed to flush"),
    }
}
