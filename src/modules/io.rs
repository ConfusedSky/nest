use wren_macros::{foreign, foreign_static_method};

use super::{source_file, Class, Module};
use std::io::{stdout, Write};

pub fn init_module<'wren>() -> Module<'wren> {
    let mut stdout_class = Class::new();
    stdout_class
        .static_methods
        .insert("flush()", foreign!(flush));

    let mut io_module = Module::new(source_file!("io.wren"));
    io_module.classes.insert("Stdout", stdout_class);

    io_module
}

#[foreign_static_method]
fn flush() -> Result<(), &'static str> {
    stdout().flush().map_err(|_| "Stdout failed to flush")
}
