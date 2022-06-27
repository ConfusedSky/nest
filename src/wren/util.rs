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

pub mod macro_helper {
    #[macro_export]
    macro_rules! make_args {
        ($class:ident, $($args:tt),+) => {
            &(&$class, $( &$args )+)
        };
        ($class:ident) => {
            &(&$class)
        };
    }
    pub use make_args;
}

#[macro_export]
macro_rules! make_call {
        ($vm:ident { $class:ident.$handle:ident() }) => {{
            use crate::wren::util::{make_call_helper, macro_helper};
            make_call_helper($vm, &$handle, (macro_helper::make_args!($class)))
        }};
        ($vm:ident { $class:ident.$handle:ident($($args:expr),+ ) }) => {{
            use crate::wren::util::{make_call_helper, macro_helper};
            make_call_helper($vm, &$handle, (macro_helper::make_args!($class, $($args),+)))
        }};
    }
pub use make_call;