use std::{
    cell::RefCell,
    ffi::{c_void, CStr, CString},
    mem::MaybeUninit,
    pin::Pin,
};

use crate::wren_sys;

unsafe fn get_user_data<'s, V>(vm: *mut wren_sys::WrenVM) -> Option<&'s mut V> {
    let user_data = wren_sys::wrenGetUserData(vm);
    if user_data.is_null() {
        None
    } else {
        Some(user_data.cast::<V>().as_mut().unwrap())
    }
}

unsafe extern "C" fn write_fn<V: VmUserData>(vm: *mut wren_sys::WrenVM, text: *const i8) {
    let text = CStr::from_ptr(text).to_string_lossy();

    let user_data = get_user_data::<V>(vm);
    if let Some(user_data) = user_data {
        user_data.on_write(text.as_ref());
    }
}

unsafe extern "C" fn error_fn<V: VmUserData>(
    vm: *mut wren_sys::WrenVM,
    error_type: wren_sys::WrenErrorType,
    module: *const i8,
    line: i32,
    msg: *const i8,
) {
    let module = CStr::from_ptr(module).to_string_lossy();
    let msg = CStr::from_ptr(msg).to_string_lossy();
    let kind = match error_type {
        wren_sys::WrenErrorType_WREN_ERROR_COMPILE => ErrorKind::Compile {
            module: module.as_ref(),
            line,
            msg: msg.as_ref(),
        },
        wren_sys::WrenErrorType_WREN_ERROR_RUNTIME => ErrorKind::Runtime(msg.as_ref()),
        wren_sys::WrenErrorType_WREN_ERROR_STACK_TRACE => ErrorKind::Stacktrace {
            module: module.as_ref(),
            line,
            msg: msg.as_ref(),
        },
        _ => panic!("Should never reach here"),
    };

    let user_data = get_user_data::<V>(vm);
    if let Some(user_data) = user_data {
        user_data.on_error(kind);
    }
}

pub enum ErrorKind<'s> {
    Compile {
        module: &'s str,
        line: i32,
        msg: &'s str,
    },
    Runtime(&'s str),
    Stacktrace {
        module: &'s str,
        line: i32,
        msg: &'s str,
    },
}

pub enum ResultErrorKind {
    Compile,
    Runtime,
}

#[allow(unused_variables)]
// We define empty defaults here so that the user can define what they want
pub trait VmUserData {
    fn on_write(&mut self, text: &str) {}
    fn on_error(&mut self, kind: ErrorKind) {}
}

pub struct Vm<V> {
    vm: *mut wren_sys::WrenVM,
    // This value is held here so that it is
    // disposed of properly when execution is finished
    // but it isn't actually used in the struct
    _user_data: Pin<Box<RefCell<V>>>,
}

impl<V> Drop for Vm<V> {
    fn drop(&mut self) {
        unsafe { wren_sys::wrenFreeVM(self.vm) }
    }
}

impl<V> Vm<V>
where
    V: VmUserData,
{
    pub fn new(user_data: V) -> Self {
        unsafe {
            let mut config: wren_sys::WrenConfiguration = MaybeUninit::zeroed().assume_init();
            wren_sys::wrenInitConfiguration(&mut config);

            let user_data = Box::pin(RefCell::new(user_data));

            config.writeFn = Some(write_fn::<V>);
            config.errorFn = Some(error_fn::<V>);
            config.userData = user_data.as_ptr().cast::<c_void>();

            let vm = wren_sys::wrenNewVM(&mut config);

            Self {
                vm,
                _user_data: user_data,
            }
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
