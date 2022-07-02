#![allow(unsafe_code)]

pub mod context;
mod fiber;
mod foreign;
mod handle;
mod system_methods;
#[cfg(test)]
mod test;
pub mod user_data;
mod value;

pub use fiber::Fiber;
pub use handle::{CallHandle, Handle};
pub use user_data::UserData as VmUserData;
pub use value::{GetArgs, GetValue, SetArgs, SetValue};

pub use wren_sys::WREN_VERSION_STRING as VERSION;

use std::{ffi::c_void, marker::PhantomData, mem::ManuallyDrop};

pub use self::{
    context::{Context, RawForeign as RawForeignContext, RawNative as RawNativeContext},
    system_methods::SystemMethods,
};
use wren_sys::{self as ffi, WrenErrorType, WrenInterpretResult, WrenVM};

pub type ForeignMethod<'wren, T> = unsafe fn(vm: Context<'wren, T, context::Foreign>);
pub type Result<T> = std::result::Result<T, InterpretResultErrorKind>;

#[macro_export]
macro_rules! cstr {
    ($s:expr) => {{
        use std::ffi::CStr;
        const CSTR: *const i8 =
            (concat!($s, "\0") as *const str as *const [::std::os::raw::c_char]).cast::<i8>();
        #[allow(unused_unsafe)]
        unsafe {
            CStr::from_ptr(CSTR)
        }
    }};
}
pub use cstr;

pub type Slot = std::os::raw::c_int;

#[derive(Debug)]
pub struct ErrorContext<'s> {
    pub module: &'s str,
    pub line: i32,
    pub msg: &'s str,
}

#[derive(Debug)]
pub enum ErrorKind<'s> {
    Compile(ErrorContext<'s>),
    Runtime(&'s str),
    Stacktrace(ErrorContext<'s>),
    Unknown(WrenErrorType, ErrorContext<'s>),
}

#[derive(Debug)]
pub enum InterpretResultErrorKind {
    Compile,
    Runtime,
    IncorrectNumberOfArgsPassed,
    Unknown(WrenInterpretResult),
}

impl InterpretResultErrorKind {
    const fn new_from_result(result: u32) -> Result<()> {
        match result {
            wren_sys::WrenInterpretResult_WREN_RESULT_COMPILE_ERROR => Err(Self::Compile),
            wren_sys::WrenInterpretResult_WREN_RESULT_RUNTIME_ERROR => Err(Self::Runtime),
            wren_sys::WrenInterpretResult_WREN_RESULT_SUCCESS => Ok(()),
            kind => Err(Self::Unknown(kind)),
        }
    }
}

// The values contained in here are boxed because for some reason
// we see failures otherwise
// There is probably a more efficient way to implement this
#[repr(C)]
struct SystemUserData<'wren, V: 'wren> {
    system_methods: Option<SystemMethods<'wren>>,
    // User data must always be the last item in the struct because it is variable
    // size and sometimes we need to access other system user data items from an untyped
    // context so we want to make sure that while this can grow and shrink that it doesn't
    // affect the offsets to other items in the system data
    user_data: ManuallyDrop<V>,
}

impl<'wren, V> SystemUserData<'wren, V> {
    const fn new(user_data: V) -> Self {
        Self {
            user_data: ManuallyDrop::new(user_data),
            system_methods: None,
        }
    }
}

impl<'wren, V> Drop for SystemUserData<'wren, V> {
    fn drop(&mut self) {
        // Make sure user data is dropped first because there might be things that
        // the user_data depends on that are used in system methods
        unsafe {
            ManuallyDrop::drop(&mut self.user_data);
        }
        drop(self.system_methods.take());
    }
}

pub struct Vm<'wren, V: VmUserData<'wren, V>> {
    vm: Context<'wren, V, context::Native>,
    // The user data object is actually held and owned by the vm
    // We will handle dropping this data ourselves
    _phantom: PhantomData<SystemUserData<'wren, V>>,
}

impl<'wren, V> Drop for Vm<'wren, V>
where
    V: VmUserData<'wren, V>,
{
    fn drop(&mut self) {
        // Take and drop the user data here so that all handles are
        // freed before the vm is freed
        unsafe {
            let user_data = ffi::wrenGetUserData(self.vm.as_ptr());
            // Create a new box object and let it free itself before the
            // vm is freed
            drop(Box::<SystemUserData<V>>::from_raw(user_data.cast()));
            ffi::wrenFreeVM(self.as_ptr());
        }
    }
}

impl<'wren, V> Vm<'wren, V>
where
    V: VmUserData<'wren, V>,
{
    pub fn get_context(&mut self) -> &mut Context<'wren, V, context::Native> {
        &mut self.vm
    }

    const fn as_ptr(&self) -> *mut WrenVM {
        self.vm.as_ptr()
    }

    pub fn new(user_data: V) -> Self {
        unsafe {
            let mut config = foreign::init_config::<V>();

            let user_data = SystemUserData::new(user_data);
            let user_data = Box::new(user_data);
            let user_data_ptr = Box::into_raw(user_data);

            config.userData = user_data_ptr.cast::<c_void>();

            let mut vm: Context<V, context::Native> =
                Context::new_unchecked(ffi::wrenNewVM(&mut config));
            (*user_data_ptr).system_methods = Some(SystemMethods::new(&mut vm));

            Self {
                vm,
                _phantom: PhantomData,
            }
        }
    }
}
