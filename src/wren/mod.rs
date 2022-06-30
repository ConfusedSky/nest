#![allow(unsafe_code)]

mod context;
mod fiber;
mod foreign;
mod handle;
mod system_methods;
#[cfg(test)]
mod test;
pub mod user_data;
mod util;
mod value;

pub use fiber::Fiber;
pub use handle::Handle;
pub use user_data::UserData as VmUserData;
pub use value::{Get, GetArgs, Set, SetArgs, Value};

pub use wren_sys::WREN_VERSION_STRING as VERSION;

use std::{
    ffi::{c_void, CStr, CString, FromBytesWithNulError},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use wren_sys::{self as ffi, WrenErrorType, WrenInterpretResult, WrenVM};

pub type ForeignMethod<'wren, T> = unsafe fn(vm: VmContext<'wren, T>);
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

    pub fn get_user_data(&self) -> &V {
        // SAFETY this is called from a typed context
        unsafe { &foreign::get_system_user_data(self.as_ptr()).user_data }
    }
    pub fn get_user_data_mut(&mut self) -> &mut V {
        // SAFETY this is called from a typed context
        unsafe { &mut foreign::get_system_user_data(self.as_ptr()).user_data }
    }
    pub fn get_user_data_mut_with_context(&mut self) -> (&mut V, &mut RawVMContext<'wren>) {
        unsafe {
            (
                &mut foreign::get_system_user_data(self.0.as_ptr()).user_data,
                &mut self.0,
            )
        }
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

    fn get_system_methods<'s>(&self) -> &'s SystemMethods<'wren> {
        unsafe {
            foreign::get_system_user_data::<()>(self.as_ptr())
                .system_methods
                .as_ref()
                .expect("SystemMethods should be initialized at this point")
        }
    }

    pub fn get_variable<Module, Name>(
        &mut self,
        module: Module,
        name: Name,
        slot: Slot,
    ) -> Option<Handle<'wren>>
    where
        Module: AsRef<str>,
        Name: AsRef<str>,
    {
        let module = CString::new(module.as_ref()).unwrap();
        let name = CString::new(name.as_ref()).unwrap();
        // SAFETY wrenGetVariable is definitely safe if wrenHasModule and wrenHasVariable
        // are called beforehand
        // wrenHasVariable is safe if wrenHasModule has been called
        // and wrenHasModule is always safe to call
        unsafe {
            if !ffi::wrenHasModule(self.as_ptr(), module.as_ptr())
                || !ffi::wrenHasVariable(self.as_ptr(), module.as_ptr(), name.as_ptr())
            {
                None
            } else {
                Some(self.get_variable_unchecked(module.as_c_str(), name.as_c_str(), slot))
            }
        }
    }

    /// SAFETY: this is always non null but will segfault if an invalid slot
    /// is asked for
    /// MAYBE: Will seg fault if the variable does not exist?
    /// Still need to set up module resolution
    pub unsafe fn get_variable_unchecked(
        &mut self,
        module: &CStr,
        name: &CStr,
        slot: Slot,
    ) -> Handle<'wren> {
        ffi::wrenGetVariable(self.as_ptr(), module.as_ptr(), name.as_ptr(), slot);
        Handle::get_from_vm(self, slot)
    }

    pub fn make_call_handle_slice(
        &mut self,
        signature: &[u8],
    ) -> std::result::Result<Handle<'wren>, FromBytesWithNulError> {
        let cstr = CStr::from_bytes_with_nul(signature)?;
        Ok(self.make_call_handle(cstr))
    }

    pub fn make_call_handle(&mut self, signature: &CStr) -> Handle<'wren> {
        let vm = self.0;
        unsafe {
            // SAFETY: this function is always safe to call but may be unsafe to use the handle it returns
            // as that handle might not be valid and safe to use
            let ptr = ffi::wrenMakeCallHandle(vm.as_ptr(), signature.as_ptr());
            Handle::new(self, NonNull::new_unchecked(ptr))
        }
    }

    pub fn interpret<M, S>(&self, module: M, source: S) -> Result<()>
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
    pub unsafe fn call(&mut self, method: &Handle<'wren>) -> Result<()> {
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

    // TODO: Create safe version that returns Options depending on how many slots
    // there are
    pub unsafe fn get_stack_unchecked<Args: GetArgs<'wren>>(&mut self) -> Args {
        Args::get_slots(self)
    }

    pub unsafe fn get_return_value_unchecked<Args: Get<'wren>>(&mut self) -> Args {
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
    user_data: V,
}

impl<'wren, V> SystemUserData<'wren, V> {
    const fn new(user_data: V) -> Self {
        Self {
            user_data,
            system_methods: None,
        }
    }
}

pub struct Vm<'wren, V: VmUserData<'wren, V>> {
    vm: VmContext<'wren, V>,
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
    pub fn get_context(&mut self) -> &mut VmContext<'wren, V> {
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

            let mut vm = VmContext::new_unchecked(ffi::wrenNewVM(&mut config));
            (*user_data_ptr).system_methods = Some(SystemMethods::new(&mut vm));

            Self {
                vm,
                _phantom: PhantomData,
            }
        }
    }
}
