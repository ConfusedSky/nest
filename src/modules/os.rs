use crate::wren::VmContext;
use crate::wren::VERSION;

use super::{source_file, Class, Module};
use std::env::args;
use std::env::current_dir;

pub fn init_module() -> Module {
    let mut platform_class = Class::new();
    platform_class.static_methods.insert("isPosix", is_posix);
    platform_class.static_methods.insert("name", name);
    platform_class.static_methods.insert("homePath", home_path);

    let mut process_class = Class::new();
    process_class
        .static_methods
        .insert("allArguments", all_arguments);
    process_class.static_methods.insert("version", version);
    process_class.static_methods.insert("cwd", cwd);
    process_class.static_methods.insert("pid", pid);
    process_class.static_methods.insert("ppid", ppid);

    let mut module = Module::new(source_file!("os.wren"));
    module.classes.insert("Process", process_class);
    module.classes.insert("Platform", platform_class);

    module
}

fn is_posix(vm: VmContext) {
    vm.set_return_value(&std::env::consts::OS);
}

fn name(vm: VmContext) {
    let value = std::env::consts::FAMILY == "unix";
    vm.set_return_value(&(value));
}

fn home_path(vm: VmContext) {
    let dir = dirs::home_dir();

    dir.map_or_else(
        || {
            vm.abort_fiber("Cannot get the user's home directory");
        },
        |dir| {
            vm.set_return_value(&dir.to_string_lossy().as_ref());
        },
    );
}

fn all_arguments(vm: VmContext) {
    let arguments = args().collect::<Vec<String>>();
    vm.set_return_value(&arguments);
}

fn version(vm: VmContext) {
    let version = unsafe { std::ffi::CString::from_vec_with_nul_unchecked(VERSION.to_vec()) };
    vm.set_return_value(&version);
}

fn cwd(vm: VmContext) {
    let dir = current_dir();

    if let Ok(dir) = dir {
        vm.set_return_value(&dir.to_string_lossy().as_ref());
    } else {
        vm.abort_fiber("Cannot get current working directory.");
    }
}

fn pid(vm: VmContext) {
    vm.set_return_value(&(f64::from(std::process::id())));
}

fn ppid(vm: VmContext) {
    vm.abort_fiber("Unimplemented!");
}
