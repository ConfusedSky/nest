use std::{
    ffi::{CStr, CString, FromBytesWithNulError},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use wren_sys::{self as ffi, WrenVM};

use super::{
    foreign,
    handle::CallHandle,
    system_methods::SystemMethods,
    value::{TryGetError, TryGetResult, WrenType},
    Fiber, GetArgs, GetValue, Handle, InterpretResultErrorKind, Result, SetArgs, SetValue, Slot,
    SystemUserData, VmUserData,
};

pub type Raw<'wren, L> = Context<'wren, NoTypeInfo, L>;
pub type RawForeign<'wren> = Raw<'wren, Foreign>;
pub type RawNative<'wren> = Raw<'wren, Native>;
pub type RawUnknown<'wren> = Raw<'wren, UnknownLocation>;

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct Context<'wren, T, L: Location>(
    NonNull<WrenVM>,
    PhantomData<&'wren mut SystemUserData<'wren, T>>,
    PhantomData<L>,
);

impl<'wren, V, L: Location> Context<'wren, V, L> {
    const unsafe fn transmute<V2, L2: Location>(&self) -> &Context<'wren, V2, L2> {
        &*((self as *const Context<V, L>).cast::<Context<V2, L2>>())
    }
    unsafe fn transmute_mut<V2, L2: Location>(&mut self) -> &mut Context<'wren, V2, L2> {
        &mut *((self as *mut Context<V, L>).cast::<Context<V2, L2>>())
    }

    // NOTE THESE ARE ALL DOWNCASTS SO THIS IS SAFE
    // Foreign is more restrictive than native
    // Raw is more restrictive than typed
    pub const fn as_unknown(&self) -> &Context<'wren, V, UnknownLocation> {
        unsafe { self.transmute::<V, UnknownLocation>() }
    }

    pub fn as_unknown_mut(&mut self) -> &mut Context<'wren, V, UnknownLocation> {
        unsafe { self.transmute_mut::<V, UnknownLocation>() }
    }

    pub const fn as_raw(&self) -> &Context<'wren, NoTypeInfo, L> {
        unsafe { self.transmute::<NoTypeInfo, L>() }
    }
    pub fn as_raw_mut(&mut self) -> &mut Context<'wren, NoTypeInfo, L> {
        unsafe { self.transmute_mut::<NoTypeInfo, L>() }
    }

    pub const fn as_ptr(&self) -> *mut WrenVM {
        self.0.as_ptr()
    }

    #[allow(dead_code)]
    pub(super) unsafe fn new(vm: *mut WrenVM) -> Option<Self> {
        Some(Self(NonNull::new(vm)?, PhantomData, PhantomData))
    }

    pub(super) unsafe fn new_unchecked(vm: *mut WrenVM) -> Self {
        Self(NonNull::new_unchecked(vm), PhantomData, PhantomData)
    }
}

// Type information is needed to get user data
impl<'wren, V: VmUserData<'wren, V>, L: Location> Context<'wren, V, L> {
    pub fn get_user_data(&self) -> &V {
        // SAFETY this is called from a typed context
        unsafe { &foreign::get_system_user_data(self.as_ptr()).user_data }
    }
    pub fn get_user_data_mut(&mut self) -> &mut V {
        // SAFETY this is called from a typed context
        unsafe { &mut foreign::get_system_user_data(self.as_ptr()).user_data }
    }
    pub fn get_user_data_mut_with_context(
        &mut self,
    ) -> (&mut V, &mut Context<'wren, NoTypeInfo, L>) {
        unsafe {
            (
                &mut foreign::get_system_user_data(self.0.as_ptr()).user_data,
                self.as_raw_mut(),
            )
        }
    }
}

impl<'wren, V: VmUserData<'wren, V>> Deref for Context<'wren, V, Native> {
    type Target = RawNative<'wren>;
    fn deref(&self) -> &Self::Target {
        self.as_raw()
    }
}
impl<'wren, V: VmUserData<'wren, V>> DerefMut for Context<'wren, V, Native> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_raw_mut()
    }
}

impl<'wren, V: VmUserData<'wren, V>> Deref for Context<'wren, V, Foreign> {
    type Target = RawForeign<'wren>;
    fn deref(&self) -> &Self::Target {
        self.as_raw()
    }
}
impl<'wren, V: VmUserData<'wren, V>> DerefMut for Context<'wren, V, Foreign> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_raw_mut()
    }
}

impl<'wren, V: VmUserData<'wren, V>> Deref for Context<'wren, V, UnknownLocation> {
    type Target = RawUnknown<'wren>;
    fn deref(&self) -> &Self::Target {
        self.as_raw()
    }
}
impl<'wren, V: VmUserData<'wren, V>> DerefMut for Context<'wren, V, UnknownLocation> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_raw_mut()
    }
}

#[derive(Debug, PartialEq)]
pub enum CallError<'wren> {
    InterpretError(InterpretResultErrorKind),
    TryGetError(TryGetError<'wren>),
}

impl<'wren> From<InterpretResultErrorKind> for CallError<'wren> {
    fn from(e: InterpretResultErrorKind) -> Self {
        Self::InterpretError(e)
    }
}

impl<'wren> From<TryGetError<'wren>> for CallError<'wren> {
    fn from(e: TryGetError<'wren>) -> Self {
        Self::TryGetError(e)
    }
}

pub type CallResult<'wren, T> = std::result::Result<T, CallError<'wren>>;

// Calling can only happen from a native context
impl<'wren, T> Context<'wren, T, Native> {
    pub fn interpret<M, S>(&self, module: M, source: S) -> Result<()>
    where
        M: AsRef<str>,
        S: AsRef<str>,
    {
        // SAFETY this will segfault if it is called within a foreign function
        // so it's safe because it must be used in a native context
        unsafe {
            let module = CString::new(module.as_ref()).unwrap();
            let source = CString::new(source.as_ref()).unwrap();
            let result = ffi::wrenInterpret(self.as_ptr(), module.as_ptr(), source.as_ptr());

            InterpretResultErrorKind::new_from_result(result)
        }
    }

    unsafe fn _call<Args: SetArgs<'wren, Native>>(
        &mut self,
        subject: &Handle<'wren>,
        method: &CallHandle<'wren>,
        args: &Args,
    ) -> CallResult<'wren, ()> {
        let slot_count =
            <Handle as SetValue<'wren, Native>>::REQUIRED_SLOTS + Args::TOTAL_REQUIRED_SLOTS;

        // make sure there enough slots to send over the subject
        // and all it's args
        let vm = self.as_raw_mut();

        // Allocate enough space for the subject and for the args
        vm.ensure_slots(slot_count);

        // Send over the subject and all of it's args
        // Sending is always done in reverse order so
        // previous slots can be used as scratch slots
        // without needing to allocate more
        args.set_slots(vm, 1);
        subject.set_slot(vm, 0);

        let result = ffi::wrenCall(vm.as_ptr(), method.as_ptr());
        InterpretResultErrorKind::new_from_result(result)?;
        Ok(())
    }

    /// Call [method] on a [subject] with [args] on the vm
    /// subject is usually a class or an object, but all calls require a subject
    pub fn call<G: GetValue<'wren, Native>, Args: SetArgs<'wren, Native>>(
        &mut self,
        subject: &Handle<'wren>,
        method: &CallHandle<'wren>,
        args: &Args,
    ) -> CallResult<'wren, G> {
        if method.get_argument_count() == Args::COUNT {
            unsafe {
                self._call(subject, method, args)?;
                self.as_raw_mut().get_return_value().map_err(Into::into)
            }
        } else {
            Err(InterpretResultErrorKind::IncorrectNumberOfArgsPassed.into())
        }
    }

    /// Call [method] on a [subject] with [args] on the vm
    /// subject is usually a class or an object, but all calls require a subject
    /// otherwise it's UB
    /// Arguments must be set up correctly as well
    pub unsafe fn call_unchecked<G: GetValue<'wren, Native>, Args: SetArgs<'wren, Native>>(
        &mut self,
        subject: &Handle<'wren>,
        method: &CallHandle<'wren>,
        args: &Args,
    ) -> CallResult<'wren, G> {
        self._call(subject, method, args)?;

        // This should be safe as long as the type is set correctly
        Ok(self.as_raw_mut().get_return_value_unchecked::<G>())
    }

    /// Checks a handle to see if it is a valid fiber, if it is return the handle as a fiber
    /// in the ok varient, otherwise returns the original handle
    pub fn check_fiber(&mut self, handle: Handle<'wren>) -> TryGetResult<'wren, Fiber<'wren>> {
        Fiber::try_from_handle(self, handle)
    }
}

impl<'wren> Context<'wren, NoTypeInfo, Foreign> {
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

impl<'wren, L: Location> Context<'wren, NoTypeInfo, L> {
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
                self.ensure_slots(1);
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
        Handle::get_slot_unchecked(self, slot)
    }

    pub fn make_call_handle_slice(
        &mut self,
        signature: &[u8],
    ) -> std::result::Result<CallHandle<'wren>, FromBytesWithNulError> {
        CallHandle::new_from_slice(self.as_unknown_mut(), signature)
    }

    pub fn make_call_handle(&mut self, signature: &CStr) -> CallHandle<'wren> {
        CallHandle::new_from_signature(self.as_unknown_mut(), signature)
    }

    pub unsafe fn get_stack_unchecked<Args: GetArgs<'wren, L>>(&mut self) -> Args {
        Args::get_slots_unchecked(self)
    }

    pub unsafe fn get_return_value_unchecked<Args: GetValue<'wren, L>>(&mut self) -> Args {
        Args::get_slots_unchecked(self)
    }

    pub fn get_stack<Args: GetArgs<'wren, L>>(&mut self) -> Args::TryGetTarget {
        Args::try_get_slots(self, false)
    }

    pub fn get_return_value<Args: GetValue<'wren, L>>(&mut self) -> TryGetResult<'wren, Args> {
        Args::try_get_slots(self, false)
    }

    pub fn set_stack<Args: SetArgs<'wren, L>>(&mut self, args: &Args) {
        args.set_wren_stack(self, 0);
    }

    pub fn set_return_value<Args: SetValue<'wren, L>>(&mut self, arg: &Args) {
        arg.set_wren_stack(self, 0);
    }

    /// It is unclear how safe this one is now, since increasing the
    /// slots seems to have lead to a bug
    pub unsafe fn ensure_slots(&mut self, num_slots: Slot) {
        wren_sys::wrenEnsureSlots(self.as_ptr(), num_slots);
    }

    pub(super) unsafe fn get_slot_type(&self, slot: Slot) -> WrenType {
        let t = ffi::wrenGetSlotType(self.as_ptr(), slot);
        WrenType::from(t)
    }

    pub fn get_slot_count(&self) -> Slot {
        // This call should always be safe, since it doesn't
        // modify any state
        unsafe { ffi::wrenGetSlotCount(self.as_ptr()) }
    }

    // Note this is only valid till the &mut call so
    // it is represented as a immutable reference
    // That way the vm can't be used while a reference is held
    // to the types
    pub fn get_stack_types(&self) -> Vec<WrenType> {
        unsafe {
            let slot_count = self.get_slot_count();
            let mut stack_values = Vec::new();

            for i in 0..slot_count {
                stack_values.push(self.get_slot_type(i));
            }

            stack_values
        }
    }
}

#[derive(Clone)]
pub struct NoTypeInfo;

mod sealed {
    use super::{Foreign, Native, UnknownLocation};

    pub trait Location {}
    impl Location for Foreign {}
    impl Location for Native {}
    impl Location for UnknownLocation {}
}

pub trait Location: sealed::Location {}
#[derive(Clone)]
pub struct Foreign;
impl Location for Foreign {}
#[derive(Clone)]
pub struct Native;
impl Location for Native {}

#[derive(Clone)]
pub struct UnknownLocation;
impl Location for UnknownLocation {}

mod assert {
    use super::{Context, Native, VmUserData, WrenVM};

    struct T;
    impl<'wren> VmUserData<'wren, Self> for T {}
    // Ensure that VMPtr is the same Size as `*mut WrenVM`
    // the whole purpose of it is to make it easier to access
    // the wren api, without having to sacrifice size, performance or ergonomics
    // So they should be directly castable
    static_assertions::assert_eq_align!(Context<T, Native>, *mut WrenVM);
    static_assertions::assert_eq_size!(Context<T, Native>, *mut WrenVM);
}
