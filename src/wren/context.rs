use std::{
    ffi::{CStr, CString, FromBytesWithNulError},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use wren_sys::{self as ffi, WrenVM};

use super::{
    foreign, system_methods::SystemMethods, Get, GetArgs, Handle, InterpretResultErrorKind, Result,
    Set, SetArgs, Slot, SystemUserData, VmUserData,
};

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Raw<'wren>(NonNull<WrenVM>, PhantomData<&'wren mut WrenVM>);
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Context<'wren, T>(
    Raw<'wren>,
    PhantomData<&'wren mut WrenVM>,
    PhantomData<&'wren mut SystemUserData<'wren, T>>,
);

mod assert {
    use super::{Context, VmUserData, WrenVM};

    struct T;
    impl<'wren> VmUserData<'wren, Self> for T {}
    // Ensure that VMPtr is the same Size as `*mut WrenVM`
    // the whole purpose of it is to make it easier to access
    // the wren api, without having to sacrifice size, performance or ergonomics
    // So they should be directly castable
    static_assertions::assert_eq_align!(Context<T>, *mut WrenVM);
    static_assertions::assert_eq_size!(Context<T>, *mut WrenVM);
}

impl<'wren, V: VmUserData<'wren, V>> Context<'wren, V> {
    #[allow(dead_code)]
    unsafe fn new(vm: *mut WrenVM) -> Option<Self> {
        Some(Self(Raw::new(vm)?, PhantomData, PhantomData))
    }

    pub(super) unsafe fn new_unchecked(vm: *mut WrenVM) -> Self {
        Self(Raw::new_unchecked(vm), PhantomData, PhantomData)
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
    pub fn get_user_data_mut_with_context(&mut self) -> (&mut V, &mut Raw<'wren>) {
        unsafe {
            (
                &mut foreign::get_system_user_data(self.0.as_ptr()).user_data,
                &mut self.0,
            )
        }
    }
}

impl<'wren, V> From<Context<'wren, V>> for Raw<'wren> {
    fn from(other: Context<'wren, V>) -> Self {
        other.0
    }
}

impl<'wren, V> Deref for Context<'wren, V> {
    type Target = Raw<'wren>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'wren, V> DerefMut for Context<'wren, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'wren> Raw<'wren> {
    pub(super) const fn as_ptr(&self) -> *mut WrenVM {
        self.0.as_ptr()
    }

    unsafe fn new(vm: *mut WrenVM) -> Option<Self> {
        Some(Self(NonNull::new(vm)?, PhantomData))
    }

    unsafe fn new_unchecked(vm: *mut WrenVM) -> Self {
        Self(NonNull::new_unchecked(vm), PhantomData)
    }

    pub(super) fn get_system_methods<'s>(&self) -> &'s SystemMethods<'wren> {
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
