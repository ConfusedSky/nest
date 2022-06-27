use super::{fiber, Handle, RawVMContext};

pub struct SystemMethods<'wren> {
    object_to_string: Handle<'wren>,
    object_is: Handle<'wren>,
    fiber_methods: fiber::Methods<'wren>,
}

impl<'wren> SystemMethods<'wren> {
    pub fn new(vm: &mut RawVMContext<'wren>) -> Self {
        Self {
            object_to_string: super::make_call_handle!(vm, "toString"),
            object_is: super::make_call_handle!(vm, "is(_)"),
            fiber_methods: fiber::Methods::new(vm),
        }
    }
}
