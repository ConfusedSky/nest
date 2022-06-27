use super::{Handle, Vm, VmUserData};

pub struct UserData;
impl<'wren> VmUserData<'wren, Self> for UserData {}

#[macro_export]
macro_rules! call_test_case {
        ($type:ty, $vm:ident { $class:ident.$handle:ident() } == $res:expr) => {
            assert!({
                let res: $type = crate::wren::util::make_call!($vm {$class.$handle() });
                res == $res
            })
        };
        ($type:ty, $vm:ident { $class:ident.$handle:ident($($args:expr),+ ) } == $res:expr) => {
            assert!({
                let res: $type = crate::wren::util::make_call!($vm { $class.$handle($($args),+ ) });
                res == $res
            })
        };
    }

pub use call_test_case;

pub fn create_test_vm<'wren>(source: &str) -> (Vm<'wren, UserData>, Handle<'wren>) {
    let mut vm = Vm::new(UserData);

    let vmptr = vm.get_context();
    vmptr
        .interpret("<test>", source)
        .expect("Code should run successfully");

    vmptr.ensure_slots(1);
    let class = unsafe { vmptr.get_variable_unchecked("<test>", "Test", 0) };

    (vm, class)
}
