use crate::wren::VMPtr;

use super::{Class, Module};
use std::{env::args, ffi::CString};

pub fn init_module() -> Module {
    let module_source = include_str!("os.wren");

    let mut process_class = Class::new();
    process_class
        .static_methods
        .insert("allArguments", all_arguments);

    let mut module = Module::new(CString::new(module_source).unwrap());
    module.classes.insert("Process", process_class);

    module
}

fn all_arguments(vm: VMPtr) {
    let arguments = args().collect::<Vec<String>>();
    vm.set_stack(&arguments);
}
