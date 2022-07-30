use std::{
    any::Any,
    ffi::{CStr, CString, FromBytesWithNulError, NulError},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use wren_sys::{self as ffi, WrenVM};

use crate::foreign::create_new_foreign;

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
        &*((self as *const Context<'_, V, L>).cast::<Context<'_, V2, L2>>())
    }
    unsafe fn transmute_mut<V2, L2: Location>(&mut self) -> &mut Context<'wren, V2, L2> {
        &mut *((self as *mut Context<'_, V, L>).cast::<Context<'_, V2, L2>>())
    }

    // NOTE THESE ARE ALL DOWNCASTS SO THIS IS SAFE
    // Foreign is more restrictive than native
    // Raw is more restrictive than typed
    #[must_use]
    pub const fn as_unknown(&self) -> &Context<'wren, V, UnknownLocation> {
        unsafe { self.transmute::<V, UnknownLocation>() }
    }

    #[must_use]
    pub fn as_unknown_mut(&mut self) -> &mut Context<'wren, V, UnknownLocation> {
        unsafe { self.transmute_mut::<V, UnknownLocation>() }
    }

    #[must_use]
    pub const fn as_raw(&self) -> &Context<'wren, NoTypeInfo, L> {
        unsafe { self.transmute::<NoTypeInfo, L>() }
    }

    #[must_use]
    pub fn as_raw_mut(&mut self) -> &mut Context<'wren, NoTypeInfo, L> {
        unsafe { self.transmute_mut::<NoTypeInfo, L>() }
    }

    #[must_use]
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
    #[must_use]
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

/// Calling can only happen from a native context
impl<'wren, T> Context<'wren, T, Native> {
    /// Interprets some `source` code as as module named `module`
    /// # Errors
    /// Can return an `InterpretResultErrorKind` if the source was invalid or it produced a
    /// runtime error when it was interpretted
    /// # Panics
    /// This function can panic if `module` or `source` have internal NUL values
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
    ) -> Result<()> {
        let slot_count =
            <Handle<'wren> as SetValue<'wren, Native>>::REQUIRED_SLOTS + Args::TOTAL_REQUIRED_SLOTS;

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

    /// Call `method` on a `subject` with `args` on the vm
    /// subject is usually a class or an object, but all calls require a subject
    /// # Errors
    /// This will return a result that returns errors if there is an incorrect number
    /// of arguments, an incorrect return value type or the standard interpret result
    pub fn try_call<G: GetValue<'wren, Native>, Args: SetArgs<'wren, Native>>(
        &mut self,
        subject: &Handle<'wren>,
        method: &CallHandle<'wren>,
        args: &Args,
    ) -> CallResult<'wren, G> {
        if method.get_argument_count() == Args::COUNT {
            unsafe {
                self._call(subject, method, args)?;
                self.as_raw_mut().try_get_return_value().map_err(Into::into)
            }
        } else {
            Err(InterpretResultErrorKind::IncorrectNumberOfArgsPassed.into())
        }
    }

    /// Call 'method' on a 'subject' with 'args' on the vm
    /// subject is usually a class or an object, but all calls require a subject
    /// otherwise it's UB
    /// Arguments must be set up correctly as well
    /// # Errors
    /// Returns the standard interpret error see `context.interpret`
    /// # Panics
    /// This function can panic if return value isn't able to be converted from
    /// the wren value
    /// # Safety
    /// This does not check the number of arguments passed which can cause UB if
    /// the wrong number of args are passed
    pub unsafe fn call<G: GetValue<'wren, Native>, Args: SetArgs<'wren, Native>>(
        &mut self,
        subject: &Handle<'wren>,
        method: &CallHandle<'wren>,
        args: &Args,
    ) -> Result<G> {
        self._call(subject, method, args)?;

        // This should be safe as long as the type is set correctly
        Ok(self.as_raw_mut().get_return_value::<G>())
    }

    /// Call 'method' on a 'subject' with 'args' on the vm
    /// subject is usually a class or an object, but all calls require a subject
    /// otherwise it's UB
    /// Arguments must be set up correctly as well
    /// # Errors
    /// Returns the standard interpret error see `context.interpret`
    /// # Panics
    /// This function can panic if `assume_slot_type` can't be converted to a value of type `G`
    /// # Safety
    /// This function assumes that you pass the correct number of arguments and that `assume_slot_type`
    /// is the type that is returned from `method`
    /// Otherwise this is undefined behavior
    pub unsafe fn call_unchecked<G: GetValue<'wren, Native>, Args: SetArgs<'wren, Native>>(
        &mut self,
        subject: &Handle<'wren>,
        method: &CallHandle<'wren>,
        args: &Args,
        assume_slot_type: WrenType,
    ) -> Result<G> {
        self._call(subject, method, args)?;

        // This should be safe as long as the type is set correctly
        Ok(self
            .as_raw_mut()
            .get_return_value_unchecked::<G>(assume_slot_type))
    }

    /// Checks a handle to see if it is a valid fiber, if it is return the handle as a fiber
    /// in the ok varient, otherwise returns the original handle
    /// # Errors
    /// Return a `TryGetResult` which returns `TryGetError::IncompatibleType(Some)` if
    /// `handle` isn't a fiber
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

    /// Returns a handle to a wren variable if a variable `name` exists in `module`
    /// # Panics
    /// This function can panic if `module` or `name` have internal NUL values
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
        // Safety: wrenGetVariable is definitely safe if wrenHasModule and wrenHasVariable
        // are called beforehand
        // wrenHasVariable is safe if wrenHasModule has been called
        // and wrenHasModule is always safe to call
        unsafe {
            if !ffi::wrenHasModule(self.as_ptr(), module.as_ptr())
                || !ffi::wrenHasVariable(self.as_ptr(), module.as_ptr(), name.as_ptr())
            {
                None
            } else {
                self.ensure_slots(slot);
                Some(self.get_variable_unchecked(module.as_c_str(), name.as_c_str(), slot))
            }
        }
    }

    /// # Safety
    /// this is always non null but will segfault if an invalid slot
    /// is asked for
    /// # Maybe
    /// Will seg fault if the variable does not exist?
    /// Still need to set up module resolution
    pub unsafe fn get_variable_unchecked(
        &mut self,
        module: &CStr,
        name: &CStr,
        slot: Slot,
    ) -> Handle<'wren> {
        ffi::wrenGetVariable(self.as_ptr(), module.as_ptr(), name.as_ptr(), slot);
        Handle::get_slot(self, slot)
    }

    /// # Errors
    /// If passed in slice has interior NUL bytes or isn't null terminated
    /// this will return a `FromBytesWithNulError`
    pub fn make_call_handle_slice(
        &mut self,
        signature: &[u8],
    ) -> std::result::Result<CallHandle<'wren>, FromBytesWithNulError> {
        CallHandle::new_from_slice(self.as_unknown_mut(), signature)
    }

    pub fn make_call_handle(&mut self, signature: &CStr) -> CallHandle<'wren> {
        CallHandle::new_from_signature(self.as_unknown_mut(), signature)
    }

    /// # Errors
    /// If passed in string has interior NUL bytes
    /// this will return a `NulError`
    pub fn make_call_handle_str<S: AsRef<str>>(
        &mut self,
        signature: S,
    ) -> std::result::Result<CallHandle<'wren>, NulError> {
        let cstr = CString::new(signature.as_ref())?;
        Ok(self.make_call_handle(&cstr))
    }

    /// Gets values off the stack matching the types passed in for `Args`
    /// # Panics
    /// If a value on the wren stack isn't convertable to a value in `Args`
    /// # Safety
    /// Calling this function when the number of slots hasn't been ensured is
    /// undefined behavior
    pub unsafe fn get_stack<Args: GetArgs<'wren, L>>(&mut self) -> Args {
        Args::get_slots(self)
    }

    /// Gets the value currently in slot 0 in the wren stack
    /// # Panics
    /// If the wrong return type is specified this may panic
    /// or if called in a context where there is no values on the wren stack
    pub fn get_return_value<Args: GetValue<'wren, L>>(&mut self) -> Args {
        assert!(
            self.get_slot_count() > 0,
            "get_return_value called in a context where there are no vm slots!"
        );
        // Safety
        // If the return value isn't what is expected then this should panic
        // or return an invalid value because unchecked at least checks the types
        unsafe { Args::get_slot(self, 0) }
    }

    /// Gets the value currently in slot 0 in the wren stack and assumes it's of type `ty`
    /// # Panics
    /// if `ty` can't be converted to `Args`
    /// # Safety
    /// If there are no slots on the stack or if `ty` is a different type to what
    /// is on the wren stack it is undefined behavior
    pub unsafe fn get_return_value_unchecked<Args: GetValue<'wren, L>>(
        &mut self,
        ty: WrenType,
    ) -> Args {
        Args::get_slot_unchecked(self, 0, ty)
    }

    pub fn try_get_stack<Args: GetArgs<'wren, L>>(&mut self) -> Args::TryGetTarget {
        Args::try_get_slots(self, false)
    }

    /// # Errors
    /// Can return an error if the value at slot 0 isn't convertable to `args`
    pub fn try_get_return_value<Args: GetValue<'wren, L>>(&mut self) -> TryGetResult<'wren, Args> {
        Args::try_get_slots(self, false)
    }

    pub fn set_stack<Args: SetArgs<'wren, L>>(&mut self, args: &Args) {
        args.set_wren_stack(self, 0);
    }

    pub fn set_return_value<Args: SetValue<'wren, L>>(&mut self, arg: &Args) {
        arg.set_wren_stack(self, 0);
    }

    /// # Safety
    /// this must be called with a corresponding T and class handle
    pub unsafe fn create_new_foreign<T: Any>(&mut self, class_handle: &Handle<'wren>, value: T) {
        class_handle.set_slot(self, 0);
        create_new_foreign(self.as_ptr(), value);
    }

    /// It is unclear how safe this one is now, since increasing the
    /// slots seems to have lead to a bug
    /// # Safety
    /// Not sure why this isn't safe, but calling it too often with too large
    /// a number can cause memory corruption
    ///
    /// # TODO
    /// Looks like this should only be called from a foreign context
    /// Unfortunately this isn't as simple as just moving it to
    /// the native context because lots of foreign code requires
    /// ensuring slots so this is going to be a bigger project
    /// [wren-lang/wren#1089](https://github.com/wren-lang/wren/issues/1089)
    pub unsafe fn ensure_slots(&mut self, num_slots: Slot) {
        wren_sys::wrenEnsureSlots(self.as_ptr(), num_slots);
    }

    /// # Safety
    /// This is unsafe to call on an invalid slot
    #[must_use]
    pub unsafe fn get_slot_type(&self, slot: Slot) -> WrenType {
        let t = ffi::wrenGetSlotType(self.as_ptr(), slot);
        WrenType::from(t)
    }

    #[must_use]
    pub fn get_slot_count(&self) -> Slot {
        // This call should always be safe, since it doesn't
        // modify any state
        unsafe { ffi::wrenGetSlotCount(self.as_ptr()) }
    }
}

pub trait ForeignCallOutput: Sized {
    type Output;
    fn to_output<T>(self, context: &mut Context<'_, T, Foreign>) -> Option<Self::Output>;
}

impl<'wren, S: AsRef<str>, Set: SetValue<'wren, Foreign>> ForeignCallOutput
    for std::result::Result<Set, S>
{
    type Output = Set;

    fn to_output<'a, T>(self, context: &mut Context<'_, T, Foreign>) -> Option<Self::Output> {
        match self {
            Ok(v) => Some(v),
            Err(s) => {
                context.as_raw_mut().abort_fiber(s);
                None
            }
        }
    }
}

impl<'wren, S: SetValue<'wren, Foreign>> ForeignCallOutput for S {
    type Output = S;

    fn to_output<T>(self, _context: &mut Context<'_, T, Foreign>) -> Option<Self::Output> {
        Some(self)
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
    static_assertions::assert_eq_align!(Context<'_, T, Native>, *mut WrenVM);
    static_assertions::assert_eq_size!(Context<'_, T, Native>, *mut WrenVM);
}
