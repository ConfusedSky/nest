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

unsafe fn all_arguments(vm: VMPtr) {
    vm.ensure_slots(2);
    vm.set_slot_new_list_unchecked(0);

    for arg in args() {
        vm.set_slot_string_unchecked(1, arg);
        vm.insert_in_list(0, -1, 1);
    }
}
