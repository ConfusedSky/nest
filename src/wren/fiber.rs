use std::ops::Deref;

use enumflags2::make_bitflags;

use super::{
    context::{CallResult, Context, Location, Native, Raw},
    handle::CallHandle,
    value::{TryGetError, TryGetResult, WrenType},
    GetValue, Handle, RawNativeContext, SetValue,
};

pub struct Methods<'wren> {
    // This method resumes a fiber that is suspended waiting on an asynchronous
    // operation. The first resumes it with zero arguments, and the second passes
    // one.
    transfer: CallHandle<'wren>,
    transfer_with_arg: CallHandle<'wren>,
    transfer_error: CallHandle<'wren>,
    fiber_class: Handle<'wren>,
}

impl<'wren> Methods<'wren> {
    pub(crate) fn new(vm: &mut RawNativeContext<'wren>) -> Self {
        use super::cstr;
        let transfer = vm.make_call_handle(cstr!("transfer()"));
        let transfer_with_arg = vm.make_call_handle(cstr!("transfer(_)"));
        let transfer_error = vm.make_call_handle(cstr!("transferError(_)"));

        vm.interpret("<fiber-test>", "var out = Fiber")
            .expect("Fiber class initialize failure");

        unsafe {
            vm.ensure_slots(1);
        }
        let fiber_class = vm
            .get_variable("<fiber-test>", "out", 0)
            .expect("Should be able to extract Fiber class");

        Self {
            transfer,
            transfer_with_arg,
            transfer_error,
            fiber_class,
        }
    }

    pub(crate) unsafe fn construct_unchecked(
        &'wren self,
        raw_handle: Handle<'wren>,
    ) -> Fiber<'wren> {
        Fiber {
            methods: self,
            handle: raw_handle,
        }
    }

    /// Try to construct a fiber from a handle, if it's not a valid fiber
    /// then return the original handle as an error
    pub(crate) fn construct(
        &'wren self,
        vm: &mut RawNativeContext<'wren>,
        raw_handle: Handle<'wren>,
    ) -> TryGetResult<'wren, Fiber<'wren>> {
        let is_fiber: bool = {
            let is = &vm.get_system_methods().object_is;
            vm.call(&raw_handle, is, &(&self.fiber_class))
                .expect("is should never fail for a valid wren handle")
        };

        if is_fiber {
            Ok(unsafe { self.construct_unchecked(raw_handle) })
        } else {
            Err(TryGetError::IncompatibleType(Some(raw_handle)))
        }
    }
}

pub struct Fiber<'wren> {
    methods: &'wren Methods<'wren>,
    handle: Handle<'wren>,
}

impl<'wren> Fiber<'wren> {
    pub fn try_from_handle<V>(
        vm: &mut Context<'wren, V, Native>,
        handle: Handle<'wren>,
    ) -> TryGetResult<'wren, Self> {
        vm.as_raw()
            .get_system_methods()
            .fiber_methods
            .construct(vm.as_raw_mut(), handle)
    }

    pub fn transfer<G: GetValue<'wren, Native>>(
        self,
        context: &mut RawNativeContext<'wren>,
    ) -> CallResult<'wren, G> {
        let transfer = &self.methods.transfer;
        context.call(&self, transfer, &())
    }
    pub fn transfer_with_arg<G: GetValue<'wren, Native>, S: SetValue<'wren, Native>>(
        self,
        context: &mut RawNativeContext<'wren>,
        additional_argument: S,
    ) -> CallResult<'wren, G> {
        let transfer = &self.methods.transfer_with_arg;
        context.call(&self, transfer, &(&additional_argument))
    }

    pub fn transfer_error<S, G>(
        self,
        context: &mut RawNativeContext<'wren>,
        error: S,
    ) -> CallResult<'wren, G>
    where
        S: AsRef<str>,
        G: GetValue<'wren, Native>,
    {
        let transfer_error = &self.methods.transfer_error;
        let error = error.as_ref();
        context.call(&self, transfer_error, &(&error))
    }
}

impl<'wren> Deref for Fiber<'wren> {
    type Target = Handle<'wren>;
    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

// Getting unchecked fibers will always be unsafe
// impl<'wren, L: Location> Get<'wren, L> for Fiber<'wren> {
// unsafe fn get_from_vm(vm: &mut Raw<'wren, L>, slot: super::Slot) -> Self {
// let handle = Handle::get_from_vm(vm, slot);

// vm.get_system_methods()
// .fiber_methods
// .construct_unchecked(handle)
// }
// }

impl<'wren, L: Location> SetValue<'wren, L> for Fiber<'wren> {
    const REQUIRED_SLOTS: super::Slot = 1;
    unsafe fn set_slot(&self, vm: &mut Raw<'wren, L>, slot: super::Slot) {
        self.handle.set_slot(vm, slot);
    }
}

impl<'wren> GetValue<'wren, Native> for Fiber<'wren> {
    const COMPATIBLE_TYPES: enumflags2::BitFlags<WrenType> = make_bitflags!(WrenType::{Unknown});
    unsafe fn get_slot_raw(
        _vm: &mut Raw<'wren, Native>,
        _slot: super::Slot,
        _slot_type: WrenType,
    ) -> Self {
        panic!("Getting a fiber raw is an illigal operation");
    }
    unsafe fn get_slot_unchecked(vm: &mut Raw<'wren, Native>, slot: super::Slot) -> Self {
        let handle = Handle::get_slot_unchecked(vm, slot);

        vm.get_system_methods()
            .fiber_methods
            .construct_unchecked(handle)
    }
    unsafe fn try_get_slot_raw(
        vm: &mut Raw<'wren, Native>,
        slot: super::Slot,
        slot_type: WrenType,
        get_handle: bool,
    ) -> TryGetResult<'wren, Self>
    where
        Self: Sized,
    {
        if slot_type != WrenType::Unknown {
            return Err(TryGetError::IncompatibleType(if get_handle {
                Some(Handle::get_slot_unchecked(vm, slot))
            } else {
                None
            }));
        }

        let handle = Handle::get_slot_unchecked(vm, slot);
        vm.get_system_methods().fiber_methods.construct(vm, handle)
    }
}

#[cfg(test)]
mod test {
    use crate::wren::test::{create_test_vm, Context};
    use crate::wren::{context, cstr, Fiber, Handle};

    #[test]
    #[allow(non_snake_case)]
    fn test_construct() {
        let source = "class Test {
                static returnTrue { true }
                static returnFiber { Fiber.current }
                static returnTest { Test }
            }";

        let (mut vm, Test) = create_test_vm(source, |_| {});
        let context = vm.get_context();
        let fiber_methods = &context.get_system_methods().fiber_methods;
        let returnTrue = context.make_call_handle(cstr!("returnTrue"));
        let returnFiber = context.make_call_handle(cstr!("returnFiber"));
        let returnTest = context.make_call_handle(cstr!("returnTest"));

        // We should not be able to convert any other value but a fiber to a fiber
        let true_handle: Handle = context.call(&Test, &returnTrue, &()).unwrap();
        let true_fiber = fiber_methods.construct(context, true_handle);
        assert!(true_fiber.is_err());

        let test_handle: Handle = context.call(&Test, &returnTest, &()).unwrap();
        let test_fiber = fiber_methods.construct(context, test_handle);
        assert!(test_fiber.is_err());

        let fiber_handle: Handle = context.call(&Test, &returnFiber, &()).unwrap();
        let fiber = fiber_methods.construct(context, fiber_handle);
        assert!(fiber.is_ok());

        // Test getting directly from vm
        let true_fiber = context.call::<Fiber, _>(&Test, &returnTrue, &());
        assert!(true_fiber.is_err());

        let test_fiber = context.call::<Fiber, _>(&Test, &returnTest, &());
        assert!(test_fiber.is_err());

        let fiber = context.call::<Fiber, _>(&Test, &returnFiber, &());
        assert!(fiber.is_ok());
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_transfer() {
        unsafe fn test_await(mut vm: Context<context::Foreign>) {
            let (_, fiber) = vm.get_stack_unchecked::<((), Handle)>();
            vm.get_user_data_mut().handle = Some(fiber);
        }

        let source = "class Test {
            static testResume() {
                await(Fiber.current)
                System.print(\"Test\")
                System.print(Fiber.suspend())
                System.print(\"Test\")

                return \"From wren\"
            }
            foreign static await(fiber)
        }";

        let (mut vm, Test) = create_test_vm(source, |user_data| {
            user_data.set_static_foreign_method("await(_)", test_await);
        });
        let context = vm.get_context();
        let test_resume = context.make_call_handle(cstr!("testResume()"));

        assert!(context.get_user_data().get_output().is_empty());
        #[allow(clippy::let_unit_value)]
        context.call::<(), _>(&Test, &test_resume, &()).unwrap();
        assert_eq!(context.get_user_data().get_output(), "Test\n");
        let handle = context
            .get_user_data_mut()
            .handle
            .take()
            .expect("Fiber should have been set by await");
        let fiber = context
            .check_fiber(handle)
            .expect("Handle returned from await should have been a valid fiber");

        let ret = fiber
            .transfer_with_arg::<String, _>(context, "From Rust")
            .unwrap();
        assert_eq!(
            context.get_user_data().get_output(),
            "Test\nFrom Rust\nTest\n"
        );
        assert_eq!(ret, "From wren");
    }
}
