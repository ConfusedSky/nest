use super::{cstr, fiber, handle::CallHandle, RawNativeContext};

pub struct SystemMethods<'wren> {
    pub object_to_string: CallHandle<'wren>,
    pub object_is: CallHandle<'wren>,
    pub fiber_methods: fiber::Methods<'wren>,
}

impl<'wren> SystemMethods<'wren> {
    pub fn new(vm: &mut RawNativeContext<'wren>) -> Self {
        Self {
            object_to_string: vm.make_call_handle(cstr!("toString")),
            object_is: vm.make_call_handle(cstr!("is(_)")),
            fiber_methods: fiber::Methods::new(vm),
        }
    }
}
