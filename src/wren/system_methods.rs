use super::{cstr, fiber, Handle, Raw};

pub struct SystemMethods<'wren> {
    pub object_to_string: Handle<'wren>,
    pub object_is: Handle<'wren>,
    pub fiber_methods: fiber::Methods<'wren>,
}

impl<'wren> SystemMethods<'wren> {
    pub fn new(vm: &mut Raw<'wren>) -> Self {
        Self {
            object_to_string: vm.make_call_handle(cstr!("toString")),
            object_is: vm.make_call_handle(cstr!("is(_)")),
            fiber_methods: fiber::Methods::new(vm),
        }
    }
}
