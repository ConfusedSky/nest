#[macro_export]
macro_rules! make_call {
        ($vm:ident { $class:ident.$handle:ident() }) => {
            $vm.call_unchecked(&$class, &$handle, &())
        };
        ($vm:ident { $class:ident.$handle:ident($($args:expr),+ ) }) => {
            $vm.call_unchecked(&$class, &$handle, &($(&$args),+))
        };
    }
pub use make_call;
