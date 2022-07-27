use wren::ForeignClassMethods;

use crate::source_file;

use super::{Class, Module};

pub fn init_module<'wren>() -> Module<'wren> {
    let mut bigint_class = Class::new();
    bigint_class.foreign_class_methods = Some(ForeignClassMethods::new::<BigInt>());

    let mut module = Module::new(source_file!("bigint.wren"));
    module.classes.insert("BigInt", bigint_class);

    module
}

#[derive(Default)]
struct BigInt {
    _marker_data: u8,
}
