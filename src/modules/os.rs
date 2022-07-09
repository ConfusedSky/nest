use crate::Context;
use wren::VERSION;
use wren_macros::foreign_static_method;

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

#[foreign_static_method]
fn is_posix() -> bool {
    std::env::consts::FAMILY == "unix"
}

#[foreign_static_method]
const fn name() -> &'static str {
    std::env::consts::OS
}

fn home_path(mut vm: Context) {
    let dir = dirs::home_dir();

    match dir {
        Some(dir) => vm.set_return_value(&dir.to_string_lossy().as_ref()),
        None => vm.abort_fiber("Cannot get the user's home directory"),
    }
}

#[foreign_static_method]
fn all_arguments() -> Vec<String> {
    args().collect()
}

#[foreign_static_method]
fn version() -> std::ffi::CString {
    std::ffi::CString::from_vec_with_nul(VERSION.to_vec()).expect("Version string should be valid")
}

fn cwd(mut vm: Context) {
    let dir = current_dir();

    if let Ok(dir) = dir {
        vm.set_return_value(&dir.to_string_lossy().as_ref());
    } else {
        vm.abort_fiber("Cannot get current working directory.");
    }
}

#[foreign_static_method]
fn pid() -> f64 {
    f64::from(std::process::id())
}

fn ppid(mut vm: Context) {
    vm.abort_fiber("Unimplemented!");
}
