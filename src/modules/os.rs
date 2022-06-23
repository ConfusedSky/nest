use crate::wren::VMPtr;
use crate::wren::VERSION;

use super::{Class, Module};
use std::env::current_dir;
use std::process;
use std::{env::args, ffi::CString};

pub fn init_module() -> Module {
    let module_source = include_str!("os.wren");

    let mut process_class = Class::new();
    process_class
        .static_methods
        .insert("allArguments", all_arguments);
    process_class.static_methods.insert("version", version);
    process_class.static_methods.insert("cwd", cwd);
    process_class.static_methods.insert("pid", pid);

    let mut module = Module::new(CString::new(module_source).unwrap());
    module.classes.insert("Process", process_class);

    module
}

fn all_arguments(vm: VMPtr) {
    let arguments = args().collect::<Vec<String>>();
    vm.set_return_value(&arguments);
}

fn version(vm: VMPtr) {
    let version = unsafe { std::ffi::CString::from_vec_with_nul_unchecked(VERSION.to_vec()) };
    vm.set_return_value(&version);
}

fn cwd(vm: VMPtr) {
    let dir = current_dir();

    if let Ok(dir) = dir {
        vm.set_return_value(dir.to_string_lossy().as_ref());
    } else {
        vm.abort_fiber("Cannot get current working directory.");
    }
}

fn pid(vm: VMPtr) {
    vm.set_return_value(&(f64::from(std::process::id())));
}
