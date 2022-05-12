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

fn main() {
    unsafe {
        let mut config: wren_sys::WrenConfiguration = MaybeUninit::zeroed().assume_init();
        wren_sys::wrenInitConfiguration(&mut config);

        config.writeFn = Some(write_fn);
        config.errorFn = Some(error_fn);

        let vm = wren_sys::wrenNewVM(&mut config);

        let module = CString::new("my_module").unwrap();
        let source = CString::new("System.print(\"I am running in Rust!\")").unwrap();
        let result = wren_sys::wrenInterpret(vm, module.as_ptr(), source.as_ptr());

        match result {
            wren_sys::WrenInterpretResult_WREN_RESULT_COMPILE_ERROR => println!("COMPILE_ERROR"),
            wren_sys::WrenInterpretResult_WREN_RESULT_RUNTIME_ERROR => println!("RUNTIME_ERROR"),
            wren_sys::WrenInterpretResult_WREN_RESULT_SUCCESS => println!("SUCCESS!"),
            _ => panic!("Should never reach here"),
        }

        wren_sys::wrenFreeVM(vm);
    }
}
