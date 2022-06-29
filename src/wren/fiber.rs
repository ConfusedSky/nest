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
