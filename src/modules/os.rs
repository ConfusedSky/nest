use crate::Context;
use wren::VERSION;

use super::{source_file, Class, Module};
use std::env::args;
use std::env::current_dir;

pub fn init_module<'wren>() -> Module<'wren> {
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

fn is_posix(mut vm: Context) {
    vm.set_return_value(&std::env::consts::OS);
}

fn name(mut vm: Context) {
    let value = std::env::consts::FAMILY == "unix";
    vm.set_return_value(&(value));
}

fn home_path(mut vm: Context) {
    let dir = dirs::home_dir();

    match dir {
        Some(dir) => vm.set_return_value(&dir.to_string_lossy().as_ref()),
        None => vm.abort_fiber("Cannot get the user's home directory"),
    }
}

fn all_arguments(mut vm: Context) {
    let arguments = args().collect::<Vec<String>>();
    vm.set_return_value(&arguments);
}

fn version(mut vm: Context) {
    let version = std::ffi::CString::from_vec_with_nul(VERSION.to_vec())
        .expect("Version string should be valid");
    vm.set_return_value(&version);
}

fn cwd(mut vm: Context) {
    let dir = current_dir();

    if let Ok(dir) = dir {
        vm.set_return_value(&dir.to_string_lossy().as_ref());
    } else {
        vm.abort_fiber("Cannot get current working directory.");
    }
}

fn pid(mut vm: Context) {
    vm.set_return_value(&(f64::from(std::process::id())));
}

fn ppid(mut vm: Context) {
    vm.abort_fiber("Unimplemented!");
}
