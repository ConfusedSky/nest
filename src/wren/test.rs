use super::{Handle, Vm, VmUserData};

#[derive(Default)]
pub struct UserData {
    pub output: String,
}

impl<'wren> VmUserData<'wren, Self> for UserData {
    fn on_error(&mut self, _: super::VmContext<'wren, Self>, kind: super::ErrorKind) {
        super::default::on_error(kind);
    }
    fn on_write(&mut self, _: super::VmContext<'wren, Self>, text: &str) {
        print!("{}", text);
        self.output += text;
    }
}

#[macro_export]
macro_rules! call_test_case {
        ($type:ty, $vm:ident { $class:ident.$handle:ident } == $res:expr) => {
            let slice = wren_macros::to_signature!($handle);
            let handle = $vm.make_call_handle_slice(slice);
            let res: $type = crate::wren::util::make_call!($vm {$class.handle()}).expect(
                &format!(
                    "{}.{} is not a valid invocation",
                    stringify!($class),
                    stringify!($handle),
                )
            );
            assert_eq!( res, $res );
        };
        ($type:ty, $vm:ident { $class:ident.$handle:ident() } == $res:expr) => {
            let slice = wren_macros::to_signature!($handle());
            let handle = $vm.make_call_handle_slice(slice);
            let res: $type = crate::wren::util::make_call!($vm {$class.handle()}).expect(
                &format!(
                    "{}.{} is not a valid invocation",
                    stringify!($class),
                    stringify!($handle),
                )
            );
            assert_eq!( res, $res );
        };
        ($type:ty, $vm:ident { $class:ident.$handle:ident($($args:expr),+ ) } == $res:expr) => {
            let slice = wren_macros::to_signature!($handle($($args),+ ));
            let handle = $vm.make_call_handle_slice(slice);
            let res: $type = crate::wren::util::make_call!($vm { $class.handle($($args),+ ) }).expect(
                &format!(
                    "{}.{} is not a valid invocation",
                    stringify!($class),
                    stringify!($handle),
                )
            );
            assert_eq!( res, $res );
        };
    }

pub use call_test_case;

pub fn create_test_vm<'wren>(source: &str) -> (Vm<'wren, UserData>, Handle<'wren>) {
    let mut vm = Vm::new(UserData::default());

    let vmptr = vm.get_context();
    vmptr
        .interpret("<test>", source)
        .expect("Code should run successfully");

    vmptr.ensure_slots(1);
    let class = unsafe { vmptr.get_variable_unchecked("<test>", "Test", 0) };

    (vm, class)
}
