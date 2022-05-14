#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery, unsafe_code)]

use std::{env, fs, path::PathBuf};

use wren::VMPtr;

mod modules;
mod wren;
mod wren_sys;

struct MyUserData;

impl wren::VmUserData for MyUserData {
    fn on_error(&mut self, _: VMPtr, kind: wren::ErrorKind) {
        match kind {
            wren::ErrorKind::Compile(ctx) => {
                println!("[{} line {}] [Error] {}", ctx.module, ctx.line, ctx.msg);
            }
            wren::ErrorKind::Runtime(msg) => println!("[Runtime Error] {}", msg),
            wren::ErrorKind::Stacktrace(ctx) => {
                println!("[{} line {}] in {}", ctx.module, ctx.line, ctx.msg);
            }
            wren::ErrorKind::Unknown(kind, ctx) => {
                println!(
                    "[{} line {}] [Unkown Error {}] {}",
                    ctx.module, ctx.line, kind, ctx.msg
                );
            }
        }
    }
    fn on_write(&mut self, _: VMPtr, text: &str) {
        print!("{}", text);
    }
}

fn main() {
    // There is always the executables name which we can skip
    let module: Option<String> = env::args().nth(1);

    if module.is_none() {
        println!("Please pass in the name of a script file to get started");
        return;
    }

    let module = module.unwrap();
    let mut module_path = PathBuf::new();
    module_path.push("scripts");
    module_path.push(&module);
    module_path.set_extension("wren");

    let source = fs::read_to_string(&module_path)
        .unwrap_or_else(|_| panic!("Ensure {} is a valid module name to continue", &module));

    let user_data = MyUserData;
    let vm = wren::Vm::new(user_data).unwrap();

    let result = vm.interpret(module, source);

    match result {
        Ok(()) => println!("SUCCESS"),
        Err(wren::InterpretResultErrorKind::Compile) => println!("COMPILE_ERROR"),
        Err(wren::InterpretResultErrorKind::Runtime) => println!("RUNTIME_ERROR"),
        Err(wren::InterpretResultErrorKind::Unknown(kind)) => println!("UNKNOWN ERROR: {}", kind),
    }
}
