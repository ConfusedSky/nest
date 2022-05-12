#![allow(unsafe_code)]

use std::{
    borrow::Cow,
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
    let user_data = get_user_data::<V>(vm);

    if let Some(user_data) = user_data {
        let text = CStr::from_ptr(text).to_string_lossy();
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
    let user_data = get_user_data::<V>(vm);
    if let Some(user_data) = user_data {
        let msg = CStr::from_ptr(msg).to_string_lossy();
        // This lives outside of the if statement so that it can live long enough
        // to be passed to user_data on error
        let c_module: Cow<str>;
        // Runtime doesn't have a valid module so it will crash if it goes any further
        let kind = if error_type == wren_sys::WrenErrorType_WREN_ERROR_RUNTIME {
            ErrorKind::Runtime(msg.as_ref())
        } else {
            c_module = CStr::from_ptr(module).to_string_lossy();
            let context = ErrorContext {
                module: c_module.as_ref(),
                line,
                msg: msg.as_ref(),
            };
            match error_type {
                wren_sys::WrenErrorType_WREN_ERROR_COMPILE => ErrorKind::Compile(context),
                wren_sys::WrenErrorType_WREN_ERROR_RUNTIME => ErrorKind::Runtime(msg.as_ref()),
                wren_sys::WrenErrorType_WREN_ERROR_STACK_TRACE => ErrorKind::Stacktrace(context),
                kind => ErrorKind::Unknown(kind, context),
            }
        };

        user_data.on_error(kind);
    }
}

pub struct ErrorContext<'s> {
    pub module: &'s str,
    pub line: i32,
    pub msg: &'s str,
}

pub enum ErrorKind<'s> {
    Compile(ErrorContext<'s>),
    Runtime(&'s str),
    Stacktrace(ErrorContext<'s>),
    Unknown(wren_sys::WrenErrorType, ErrorContext<'s>),
}

pub enum InterpretResultErrorKind {
    Compile,
    Runtime,
    Unknown(wren_sys::WrenInterpretResult),
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

    pub fn interpret<M, S>(&self, module: M, source: S) -> Result<(), InterpretResultErrorKind>
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
                    Err(InterpretResultErrorKind::Compile)
                }
                wren_sys::WrenInterpretResult_WREN_RESULT_RUNTIME_ERROR => {
                    Err(InterpretResultErrorKind::Runtime)
                }
                wren_sys::WrenInterpretResult_WREN_RESULT_SUCCESS => Ok(()),
                kind => Err(InterpretResultErrorKind::Unknown(kind)),
            }
        }
    }
}
