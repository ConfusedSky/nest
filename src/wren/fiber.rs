use std::ops::Deref;

use super::{
    context::{Location, Native, Raw},
    handle::CallHandle,
    value::TryGetResult,
    Get, Handle, RawNativeContext, Result, Set, Value,
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

        vm.ensure_slots(1);
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
        let is_fiber: bool = unsafe {
            let is = &vm.get_system_methods().object_is;
            vm.call_unchecked(&raw_handle, is, &(&self.fiber_class))
                .expect("is should never fail for a valid wren handle")
        };

        if is_fiber {
            Ok(unsafe { self.construct_unchecked(raw_handle) })
        } else {
            Err(raw_handle)
        }
    }
}

pub struct Fiber<'wren> {
    methods: &'wren Methods<'wren>,
    handle: Handle<'wren>,
}

impl<'wren> Fiber<'wren> {
    pub fn transfer<G: Get<'wren, Native>>(
        self,
        context: &mut RawNativeContext<'wren>,
    ) -> Result<G> {
        let transfer = &self.methods.transfer;
        let res: G = unsafe { context.call_unchecked(&self, transfer, &())? };
        Ok(res)
    }
    pub fn transfer_with_arg<G: Get<'wren, Native>, S: Set<'wren, Native>>(
        self,
        context: &mut RawNativeContext<'wren>,
        additional_argument: S,
    ) -> Result<G> {
        let transfer = &self.methods.transfer_with_arg;
        let res: G = unsafe { context.call_unchecked(&self, transfer, &(&additional_argument))? };
        Ok(res)
    }

    pub fn transfer_error<S, G>(self, context: &mut RawNativeContext<'wren>, error: S) -> Result<G>
    where
        S: AsRef<str>,
        G: Get<'wren, Native>,
    {
        let transfer_error = &self.methods.transfer_error;
        let error = error.as_ref();
        let res: G = unsafe { context.call_unchecked(&self, transfer_error, &(&error))? };
        Ok(res)
    }
}

impl<'wren> Deref for Fiber<'wren> {
    type Target = Handle<'wren>;
    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl<'wren> Value for Fiber<'wren> {
    const ADDITIONAL_SLOTS_NEEDED: super::Slot = 0;
}

// Getting unchecked fibers will always be unsafe
impl<'wren, L: Location> Get<'wren, L> for Fiber<'wren> {
    unsafe fn get_from_vm(vm: &mut Raw<'wren, L>, slot: super::Slot) -> Self {
        let handle = Handle::get_from_vm(vm, slot);

        vm.get_system_methods()
            .fiber_methods
            .construct_unchecked(handle)
    }
}

impl<'wren, L: Location> Set<'wren, L> for Fiber<'wren> {
    unsafe fn send_to_vm(&self, vm: &mut Raw<'wren, L>, slot: super::Slot) {
        self.handle.send_to_vm(vm, slot);
    }
}

impl<'wren> Value for TryGetResult<'wren, Fiber<'wren>> {
    const ADDITIONAL_SLOTS_NEEDED: super::Slot = 0;
}

impl<'wren> Get<'wren, Native> for TryGetResult<'wren, Fiber<'wren>> {
    unsafe fn get_from_vm(vm: &mut Raw<'wren, Native>, slot: super::Slot) -> Self {
        let handle = Handle::get_from_vm(vm, slot);

        vm.get_system_methods().fiber_methods.construct(vm, handle)
    }
}

#[cfg(test)]
mod test {
    use super::Fiber;
    use crate::wren::test::{create_test_vm, Context};
    use crate::wren::value::TryGetResult;
    use crate::wren::{context, cstr, Handle};

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

        unsafe {
            // We should not be able to convert any other value but a fiber to a fiber
            let true_handle: Handle = context.call_unchecked(&Test, &returnTrue, &()).unwrap();
            let true_fiber = fiber_methods.construct(context, true_handle);
            assert!(true_fiber.is_err());

            let test_handle: Handle = context.call_unchecked(&Test, &returnTest, &()).unwrap();
            let test_fiber = fiber_methods.construct(context, test_handle);
            assert!(test_fiber.is_err());

            let fiber_handle: Handle = context.call_unchecked(&Test, &returnFiber, &()).unwrap();
            let fiber = fiber_methods.construct(context, fiber_handle);
            assert!(fiber.is_ok());

            // Test getting directly from vm
            let true_fiber: TryGetResult<Fiber> =
                context.call_unchecked(&Test, &returnTrue, &()).unwrap();
            assert!(true_fiber.is_err());

            let test_fiber: TryGetResult<Fiber> =
                context.call_unchecked(&Test, &returnTest, &()).unwrap();
            assert!(test_fiber.is_err());

            let fiber: TryGetResult<Fiber> =
                context.call_unchecked(&Test, &returnFiber, &()).unwrap();
            assert!(fiber.is_ok());
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_transfer() {
        unsafe fn test_await(mut vm: Context<context::Foreign>) {
            let (_, fiber) = vm.get_stack_unchecked::<((), Fiber)>();
            vm.get_user_data_mut().fiber = Some(fiber);
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
        unsafe {
            let _: () = context.call_unchecked(&Test, &test_resume, &()).unwrap();
        }
        assert_eq!(context.get_user_data().get_output(), "Test\n");
        let fiber = context
            .get_user_data_mut()
            .fiber
            .take()
            .expect("Fiber should have been set by await");

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
