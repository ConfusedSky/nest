#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
// #![warn(unsafe_code)]

mod wren;
mod wren_sys;

struct MyUserData;

impl wren::VmUserData for MyUserData {
    fn on_error(&mut self, kind: wren::ErrorKind) {
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
    fn on_write(&mut self, text: &str) {
        print!("{}", text);
    }
}

fn main() {
    let user_data = MyUserData;
    let vm = wren::Vm::new(user_data);
    let module = "my_module";
    let source = "System.print(\"I am running in Rust!\")";

    let result = vm.interpret(module, source);

    match result {
        Ok(()) => println!("SUCCESS"),
        Err(wren::InterpretResultErrorKind::Compile) => println!("COMPILE_ERROR"),
        Err(wren::InterpretResultErrorKind::Runtime) => println!("RUNTIME_ERROR"),
        Err(wren::InterpretResultErrorKind::Unknown(kind)) => println!("UNKNOWN ERROR: {}", kind),
    }
}
