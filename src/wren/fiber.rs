use crate::make_call;

use super::{Handle, RawVMContext};

pub struct Methods<'wren> {
    // This method resumes a fiber that is suspended waiting on an asynchronous
    // operation. The first resumes it with zero arguments, and the second passes
    // one.
    resume1: Handle<'wren>,
    resume2: Handle<'wren>,
    resume_error: Handle<'wren>,
    fiber_class: Handle<'wren>,
}

impl<'wren> Methods<'wren> {
    pub(crate) fn new(vm: &mut RawVMContext<'wren>) -> Self {
        let resume1 = super::make_call_handle!(vm, "transfer()");
        let resume2 = super::make_call_handle!(vm, "transfer(_)");
        let resume_error = super::make_call_handle!(vm, "transferError(_)");

        vm.interpret("<fiber-test>", "var out = Fiber")
            .expect("Fiber class initialize failure");

        vm.ensure_slots(1);
        let fiber_class = unsafe { vm.get_variable_unchecked("<fiber-test>", "out", 0) };

        Self {
            resume1,
            resume2,
            resume_error,
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

    pub(crate) fn construct(
        &'wren self,
        vm: &mut RawVMContext<'wren>,
        raw_handle: Handle<'wren>,
    ) -> Option<Fiber<'wren>> {
        let is_fiber: bool = unsafe {
            let is = &vm.get_system_methods().object_is;
            make_call!(vm { raw_handle.is(self.fiber_class) })
                .expect("is should never fail for a valid wren handle")
        };

        if is_fiber {
            Some(unsafe { self.construct_unchecked(raw_handle) })
        } else {
            None
        }
    }
}

pub struct Fiber<'wren> {
    methods: &'wren Methods<'wren>,
    handle: Handle<'wren>,
}

#[cfg(test)]
mod test {
    use crate::make_call;
    use crate::wren::test::create_test_vm;
    use crate::wren::{make_call_handle, Handle};

    #[test]
    #[allow(non_snake_case)]
    fn test_construct() {
        let source = "class Test {
                static returnTrue { true }
                static returnFiber { Fiber.current }
                static returnTest { Test }
            }";

        let (mut vm, Test) = create_test_vm(source);
        let context = vm.get_context();
        let fiber_methods = unsafe { &context.get_system_methods().fiber_methods };
        let returnTrue = make_call_handle!(context, "returnTrue");
        let returnFiber = make_call_handle!(context, "returnFiber");
        let returnTest = make_call_handle!(context, "returnTest");

        // We should not be able to convert any other value but a fiber to a fiber
        unsafe {
            let true_handle: Handle = make_call!(context {Test.returnTrue()}).unwrap();
            let true_fiber = fiber_methods.construct(context, true_handle);
            assert!(true_fiber.is_none());

            let test_handle: Handle = make_call!(context {Test.returnTest()}).unwrap();
            let test_fiber = fiber_methods.construct(context, test_handle);
            assert!(test_fiber.is_none());

            let fiber_handle: Handle = make_call!(context {Test.returnFiber()}).unwrap();
            let fiber = fiber_methods.construct(context, fiber_handle);
            assert!(fiber.is_some());
        }
    }
}
