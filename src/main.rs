#![deny(clippy::all)]
#![warn(clippy::pedantic, clippy::nursery)]
// #![warn(unsafe_code)]

use std::{
    ffi::{CStr, CString},
    mem::MaybeUninit,
};

mod wren_sys;

unsafe extern "C" fn write_fn(_vm: *mut wren_sys::WrenVM, text: *const i8) {
    let text = CStr::from_ptr(text).to_string_lossy();
    print!("{}", text);
}

unsafe extern "C" fn error_fn(
    _vm: *mut wren_sys::WrenVM,
    error_type: wren_sys::WrenErrorType,
    module: *const i8,
    line: i32,
    msg: *const i8,
) {
    let module = CStr::from_ptr(module).to_string_lossy();
    let msg = CStr::from_ptr(msg).to_string_lossy();
    match error_type {
        wren_sys::WrenErrorType_WREN_ERROR_COMPILE => {
            println!("[{} line {}] [Error] {}", module, line, msg);
        }
        wren_sys::WrenErrorType_WREN_ERROR_RUNTIME => println!("[Runtime Error] {}", msg),
        wren_sys::WrenErrorType_WREN_ERROR_STACK_TRACE => {
            println!("[{} line {}] in {}", module, line, msg);
        }
        _ => panic!("Should never reach here"),
    }
}

enum ResultErrorKind {
    Compile,
    Runtime,
}

struct VM {
    vm: *mut wren_sys::WrenVM,
}

impl Drop for VM {
    fn drop(&mut self) {
        unsafe { wren_sys::wrenFreeVM(self.vm) }
    }
}

impl VM {
    pub fn new() -> Self {
        unsafe {
            let mut config: wren_sys::WrenConfiguration = MaybeUninit::zeroed().assume_init();
            wren_sys::wrenInitConfiguration(&mut config);

            config.writeFn = Some(write_fn);
            config.errorFn = Some(error_fn);

            let vm = wren_sys::wrenNewVM(&mut config);

            Self { vm }
        }
    }

    pub fn interpret<M, S>(&self, module: M, source: S) -> Result<(), ResultErrorKind>
    where
        M: AsRef<str>,
        S: AsRef<str>,
    {
        unsafe {
            let module = CString::new(module.as_ref()).unwrap();
            let source = CString::new(source.as_ref()).unwrap();
            let result = wren_sys::wrenInterpret(self.vm, module.as_ptr(), source.as_ptr());

            match result {
                wren_sys::WrenInterpretResult_WREN_RESULT_COMPILE_ERROR => {
                    Err(ResultErrorKind::Compile)
                }
                wren_sys::WrenInterpretResult_WREN_RESULT_RUNTIME_ERROR => {
                    Err(ResultErrorKind::Runtime)
                }
                wren_sys::WrenInterpretResult_WREN_RESULT_SUCCESS => Ok(()),
                _ => panic!("Unknown Wren Result type"),
            }
        }
    }
}

fn main() {
    let vm = VM::new();
    let module = "my_module";
    let source = "System.print(\"I am running in Rust!\")";

    let result = vm.interpret(module, source);

    match result {
        Ok(()) => println!("SUCCESS"),
        Err(ResultErrorKind::Compile) => println!("COMPILE_ERROR"),
        Err(ResultErrorKind::Runtime) => println!("RUNTIME_ERROR"),
    }
}
