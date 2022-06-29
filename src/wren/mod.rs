#![allow(unsafe_code)]

mod fiber;
mod handle;
mod system_methods;
#[cfg(test)]
mod test;
mod util;
mod value;
pub use handle::Handle;
pub use value::{Get, GetArgs, Set, SetArgs, Value};

pub use wren_sys::WREN_VERSION_STRING as VERSION;

use std::{
    borrow::Cow,
    cell::RefCell,
    ffi::{c_void, CStr, CString},
    marker::PhantomData,
    mem::{transmute_copy, MaybeUninit},
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::{null, NonNull},
};

use wren_sys::{self as ffi, WrenConfiguration, WrenErrorType, WrenInterpretResult, WrenVM};

pub type ForeignMethod<'wren, T> = unsafe fn(vm: VmContext<'wren, T>);

unsafe fn get_system_user_data<'s, V>(vm: *mut WrenVM) -> Option<&'s mut SystemUserData<'s, V>> {
    let user_data = ffi::wrenGetUserData(vm);
    if user_data.is_null() {
        None
    } else {
        Some(user_data.cast::<SystemUserData<V>>().as_mut().unwrap())
    }
}

unsafe extern "C" fn resolve_module<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    resolver: *const i8,
    name: *const i8,
) -> *const i8 {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    user_data.map_or_else(
        || std::mem::zeroed(),
        |user_data| {
            let name = CStr::from_ptr(name).to_string_lossy();
            let resolver = CStr::from_ptr(resolver).to_string_lossy();

            let name = user_data.resolve_module(resolver.as_ref(), name.as_ref());

            match name {
                Some(name) => name.into_raw(),
                None => null(),
            }
        },
    )
}

unsafe extern "C" fn load_module<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    name: *const i8,
) -> wren_sys::WrenLoadModuleResult {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    user_data.map_or_else(
        || std::mem::zeroed(),
        |user_data| {
            let name = CStr::from_ptr(name).to_string_lossy();

            let source = user_data.load_module(name.as_ref());

            let mut result: wren_sys::WrenLoadModuleResult = std::mem::zeroed();

            if let Some(source) = source {
                // SAFETY: we use into raw here and pass in a function that frees the memory
                result.source = source.as_ptr();
            }

            result
        },
    )
}

unsafe extern "C" fn bind_foreign_method<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    module: *const i8,
    class_name: *const i8,
    is_static: bool,
    signature: *const i8,
) -> wren_sys::WrenForeignMethodFn {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    user_data.map_or_else(
        || std::mem::zeroed(),
        |user_data| {
            let module = CStr::from_ptr(module).to_string_lossy();
            let class_name = CStr::from_ptr(class_name).to_string_lossy();
            let signature = CStr::from_ptr(signature).to_string_lossy();

            let method = user_data.bind_foreign_method(
                module.as_ref(),
                class_name.as_ref(),
                is_static,
                signature.as_ref(),
            )?;

            // Safety: VMPtr is a transparent wrapper over a *mut WrenVM
            transmute_copy(&method)
        },
    )
}

unsafe extern "C" fn write_fn<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    text: *const i8,
) {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

    if let Some(user_data) = user_data {
        let text = CStr::from_ptr(text).to_string_lossy();
        user_data.on_write(VmContext::new_unchecked(vm), text.as_ref());
    }
}

unsafe extern "C" fn error_fn<'wren, V: 'wren + VmUserData<'wren, V>>(
    vm: *mut WrenVM,
    error_type: WrenErrorType,
    module: *const i8,
    line: i32,
    msg: *const i8,
) {
    let mut context: VmContext<V> = VmContext::new_unchecked(vm);
    let user_data = context.get_user_data_mut();

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

        user_data.on_error(VmContext::new_unchecked(vm), kind);
    }
}

#[macro_export]
macro_rules! cstr {
    ($s:expr) => {
        (concat!($s, "\0") as *const str as *const [::std::os::raw::c_char]).cast::<i8>()
    };
}
pub use cstr;

#[macro_export]
macro_rules! make_call_handle {
    ($vm:ident, $signature:expr) => {{
        use crate::wren::cstr;
        const SIGNATURE: *const i8 = cstr!($signature);

        #[allow(unused_unsafe)]
        unsafe {
            $vm.make_call_handle(SIGNATURE)
        }
    }};
}
pub use make_call_handle;

use self::system_methods::SystemMethods;

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct RawVMContext<'wren>(NonNull<WrenVM>, PhantomData<&'wren mut WrenVM>);

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct VmContext<'wren, T>(
    RawVMContext<'wren>,
    PhantomData<&'wren mut WrenVM>,
    PhantomData<&'wren mut SystemUserData<'wren, T>>,
);

mod assert {
    use super::{VmContext, VmUserData, WrenVM};

    struct T;
    impl<'wren> VmUserData<'wren, Self> for T {}
    // Ensure that VMPtr is the same Size as `*mut WrenVM`
    // the whole purpose of it is to make it easier to access
    // the wren api, without having to sacrifice size, performance or ergonomics
    // So they should be directly castable
    static_assertions::assert_eq_align!(VmContext<T>, *mut WrenVM);
    static_assertions::assert_eq_size!(VmContext<T>, *mut WrenVM);
}

pub type Slot = std::os::raw::c_int;

impl<'wren, V: VmUserData<'wren, V>> VmContext<'wren, V> {
    #[allow(dead_code)]
    unsafe fn new(vm: *mut WrenVM) -> Option<Self> {
        Some(Self(RawVMContext::new(vm)?, PhantomData, PhantomData))
    }

    unsafe fn new_unchecked(vm: *mut WrenVM) -> Self {
        Self(RawVMContext::new_unchecked(vm), PhantomData, PhantomData)
    }

    const fn as_ptr(&self) -> *mut WrenVM {
        self.0.as_ptr()
    }

    /// the correct type
    pub fn get_user_data<'s>(&self) -> Option<&'s V> {
        // SAFETY this is called from a typed context
        unsafe { get_system_user_data(self.as_ptr()).map(|s| &s.user_data) }
    }
    /// the correct type
    pub fn get_user_data_mut<'s>(&mut self) -> Option<&'s mut V> {
        // SAFETY this is called from a typed context
        unsafe { get_system_user_data(self.as_ptr()).map(|s| &mut s.user_data) }
    }
}

impl<'wren, V> From<VmContext<'wren, V>> for RawVMContext<'wren> {
    fn from(other: VmContext<'wren, V>) -> Self {
        other.0
    }
}

impl<'wren, V> Deref for VmContext<'wren, V> {
    type Target = RawVMContext<'wren>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'wren, V> DerefMut for VmContext<'wren, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'wren> RawVMContext<'wren> {
    const fn as_ptr(&self) -> *mut WrenVM {
        self.0.as_ptr()
    }

    unsafe fn new(vm: *mut WrenVM) -> Option<Self> {
        Some(Self(NonNull::new(vm)?, PhantomData))
    }

    unsafe fn new_unchecked(vm: *mut WrenVM) -> Self {
        Self(NonNull::new_unchecked(vm), PhantomData)
    }

    /// SAFETY: This is not guarenteed to be safe the user needs to know to input
    /// the correct type
    unsafe fn get_system_methods<'s>(&self) -> &'s SystemMethods<'wren> {
        get_system_user_data::<()>(self.as_ptr())
            .expect("user_data should have been initialized at this point")
            .system_methods
            .as_ref()
            .expect("SystemMethods should be initialized at this point")
    }

    /// SAFETY: this is always non null but will segfault if an invalid slot
    /// is asked for
    /// MAYBE: Will seg fault if the variable does not exist?
    /// Still need to set up module resolution
    pub unsafe fn get_variable_unchecked<Module, Name>(
        &mut self,
        module: Module,
        name: Name,
        slot: Slot,
    ) -> Handle<'wren>
    where
        Module: AsRef<str>,
        Name: AsRef<str>,
    {
        let module = CString::new(module.as_ref()).unwrap();
        let name = CString::new(name.as_ref()).unwrap();

        ffi::wrenGetVariable(self.as_ptr(), module.as_ptr(), name.as_ptr(), slot);

        Handle::get_from_vm(self, slot)
    }

    pub unsafe fn make_call_handle_slice(&mut self, signature: &[u8]) -> Handle<'wren> {
        let ptr = signature.as_ptr().cast::<i8>() as *mut _;
        self.make_call_handle(ptr)
    }

    pub unsafe fn make_call_handle(&mut self, signature: *const i8) -> Handle<'wren> {
        let vm = self.0;
        let ptr = ffi::wrenMakeCallHandle(vm.as_ptr(), signature);

        // SAFETY: this function is always safe to call but may be unsafe to use the handle it returns
        // as that handle might not be valid
        Handle::new(self, NonNull::new_unchecked(ptr))
    }

    pub fn interpret<M, S>(&self, module: M, source: S) -> Result<(), InterpretResultErrorKind>
    where
        M: AsRef<str>,
        S: AsRef<str>,
    {
        unsafe {
            let module = CString::new(module.as_ref()).unwrap();
            let source = CString::new(source.as_ref()).unwrap();
            let result = ffi::wrenInterpret(self.as_ptr(), module.as_ptr(), source.as_ptr());

            InterpretResultErrorKind::new_from_result(result)
        }
    }

    /// Safety: Will segfault if used with an invalid method
    pub unsafe fn call(&mut self, method: &Handle<'wren>) -> Result<(), InterpretResultErrorKind> {
        let vm = self.0;
        let result = ffi::wrenCall(vm.as_ptr(), method.as_ptr());

        InterpretResultErrorKind::new_from_result(result)
    }

    pub fn ensure_slots(&mut self, num_slots: Slot) {
        // SAFETY: this one is always safe to call even if the value is negative
        unsafe {
            wren_sys::wrenEnsureSlots(self.as_ptr(), num_slots);
        }
    }

    pub fn set_stack<Args: SetArgs<'wren>>(&mut self, args: &Args) {
        args.set_wren_stack(self);
    }

    pub fn set_return_value<Args: Set<'wren> + ?Sized>(&mut self, arg: &Args) {
        arg.set_wren_stack(self);
    }

    pub unsafe fn get_stack<Args: GetArgs<'wren>>(&mut self) -> Args {
        Args::get_slots(self)
    }

    pub unsafe fn get_return_value<Args: Get<'wren>>(&mut self) -> Args {
        Args::get_slots(self)
    }

    pub fn abort_fiber<S>(&mut self, value: S)
    where
        S: AsRef<str>,
    {
        self.set_return_value(&value.as_ref());
        unsafe {
            ffi::wrenAbortFiber(self.as_ptr(), 0);
        }
    }
}

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
    Unknown(WrenInterpretResult),
}

impl InterpretResultErrorKind {
    const fn new_from_result(result: u32) -> Result<(), Self> {
        match result {
            wren_sys::WrenInterpretResult_WREN_RESULT_COMPILE_ERROR => Err(Self::Compile),
            wren_sys::WrenInterpretResult_WREN_RESULT_RUNTIME_ERROR => Err(Self::Runtime),
            wren_sys::WrenInterpretResult_WREN_RESULT_SUCCESS => Ok(()),
            kind => Err(Self::Unknown(kind)),
        }
    }
}

#[allow(unused_variables)]
// We define empty defaults here so that the user can define what they want
pub trait VmUserData<'wren, T> {
    fn resolve_module(&mut self, resolver: &str, name: &str) -> Option<CString> {
        CString::new(name.to_string()).ok()
    }
    fn load_module(&mut self, name: &str) -> Option<&'wren CStr> {
        None
    }
    fn bind_foreign_method(
        &mut self,
        module: &str,
        classname: &str,
        is_static: bool,
        signature: &str,
    ) -> Option<ForeignMethod<'wren, T>> {
        unsafe { std::mem::zeroed() }
    }
    // Default behavior is to return a struct with fields nulled out
    // so this is fine
    fn bind_foreign_class(
        &mut self,
        module: &str,
        classname: &str,
    ) -> wren_sys::WrenForeignClassMethods {
        unsafe { std::mem::zeroed() }
    }
    fn on_write(&mut self, vm: VmContext<'wren, T>, text: &str) {}
    fn on_error(&mut self, vm: VmContext<'wren, T>, kind: ErrorKind) {}
}

struct SystemUserData<'wren, V: 'wren> {
    user_data: V,
    system_methods: Option<SystemMethods<'wren>>,
}

impl<'wren, V> SystemUserData<'wren, V> {
    const fn new(user_data: V) -> Self {
        Self {
            user_data,
            system_methods: None,
        }
    }
}

pub struct Vm<'wren, V> {
    vm: VmContext<'wren, V>,
    // This value is held here so that it is
    // disposed of properly when execution is finished
    // but it isn't actually used in the struct
    user_data: Pin<Box<RefCell<SystemUserData<'wren, V>>>>,
}

impl<'wren, V> Vm<'wren, V> {
    fn as_ptr(&self) -> *mut WrenVM {
        self.vm.as_ptr()
    }
}

impl<'wren, V> Drop for Vm<'wren, V> {
    fn drop(&mut self) {
        self.user_data.as_mut().borrow_mut().system_methods = None;
        unsafe { ffi::wrenFreeVM(self.as_ptr()) }
    }
}

impl<'wren, V> Vm<'wren, V>
where
    V: VmUserData<'wren, V>,
{
    pub fn new(user_data: V) -> Self {
        unsafe {
            let mut config: MaybeUninit<WrenConfiguration> = MaybeUninit::zeroed();
            ffi::wrenInitConfiguration(config.as_mut_ptr());
            let mut config = config.assume_init();

            // TODO: Check if this is a zst and don't allocate space if not
            let user_data = SystemUserData::new(user_data);
            let user_data = Box::pin(RefCell::new(user_data));

            config.writeFn = Some(write_fn::<V>);
            config.errorFn = Some(error_fn::<V>);
            config.loadModuleFn = Some(load_module::<V>);
            config.resolveModuleFn = Some(resolve_module::<V>);
            config.bindForeignMethodFn = Some(bind_foreign_method::<V>);
            config.userData = user_data.as_ptr().cast::<c_void>();

            let mut vm = VmContext::new_unchecked(ffi::wrenNewVM(&mut config));
            (*user_data.as_ptr()).system_methods = Some(SystemMethods::new(&mut vm));

            Self { vm, user_data }
        }
    }

    pub fn get_context(&mut self) -> &mut VmContext<'wren, V> {
        &mut self.vm
    }
}
