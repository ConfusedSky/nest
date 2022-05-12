#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
// #![warn(unsafe_code)]

mod wren;
mod wren_sys;

// unsafe extern "C" fn error_fn(
// _vm: *mut wren_sys::WrenVM,
// error_type: wren_sys::WrenErrorType,
// module: *const i8,
// line: i32,
// msg: *const i8,
// ) {
// let module = CStr::from_ptr(module).to_string_lossy();
// let msg = CStr::from_ptr(msg).to_string_lossy();
// match error_type {
// wren_sys::WrenErrorType_WREN_ERROR_COMPILE => {
// println!("[{} line {}] [Error] {}", module, line, msg);
// }
// wren_sys::WrenErrorType_WREN_ERROR_RUNTIME => println!("[Runtime Error] {}", msg),
// wren_sys::WrenErrorType_WREN_ERROR_STACK_TRACE => {
// println!("[{} line {}] in {}", module, line, msg);
// }
// _ => panic!("Should never reach here"),
// }
// }

struct MyUserData;

impl wren::VmUserData for MyUserData {
    fn on_error(&mut self, kind: wren::ErrorKind) {
        match kind {
            wren::ErrorKind::Compile { module, line, msg } => {
                println!("[{} line {}] [Error] {}", module, line, msg);
            }
            wren::ErrorKind::Runtime(msg) => println!("[Runtime Error] {}", msg),
            wren::ErrorKind::Stacktrace { module, line, msg } => {
                println!("[{} line {}] in {}", module, line, msg);
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
        Err(wren::ResultErrorKind::Compile) => println!("COMPILE_ERROR"),
        Err(wren::ResultErrorKind::Runtime) => println!("RUNTIME_ERROR"),
    }
}
