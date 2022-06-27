use super::{Get, Handle, RawVMContext, SetArgs};

pub unsafe fn make_call_helper<'wren, T: Get<'wren>, Args: SetArgs<'wren>>(
    vm: &mut RawVMContext<'wren>,
    method: &Handle<'wren>,
    args: &Args,
) -> T {
    vm.set_stack(args);
    vm.call(method).unwrap();
    vm.get_return_value::<T>()
}

#[macro_export]
macro_rules! make_call {
        ($class:ident.$handle:ident($vm:ident)) => {{
            use crate::wren::util::make_call_helper;
            make_call_helper($vm, &$handle, (make_args!($class)))
        }};
        ($class:ident.$handle:ident($vm:ident, $($args:expr),+ )) => {{
            use crate::wren::util::make_call_helper;
            make_call_helper($vm, &$handle, (make_args!($class, $($args),+)))
        }};
    }
pub use make_call;
