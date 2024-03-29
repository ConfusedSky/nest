use std::collections::HashMap;

use super::{context, ForeignMethod, Handle, Vm, VmUserData};

pub type Context<'wren, L> = super::Context<'wren, UserData<'wren>, L>;

#[derive(Default)]
pub struct UserData<'wren> {
    output: String,
    static_foreign: HashMap<&'wren str, ForeignMethod<'wren, UserData<'wren>>>,
    pub handle: Option<Handle<'wren>>,
}

impl<'wren> VmUserData<'wren, Self> for UserData<'wren> {
    fn on_error(
        &mut self,
        _: super::Context<'wren, Self, context::Foreign>,
        kind: super::ErrorKind<'_>,
    ) {
        super::user_data::on_error(kind);
    }
    fn on_write(&mut self, _: super::Context<'wren, Self, context::Foreign>, text: &str) {
        print!("{}", text);
        self.output += text;
    }
    fn bind_foreign_method(
        &mut self,
        module: &str,
        classname: &str,
        is_static: bool,
        signature: &str,
    ) -> Option<ForeignMethod<'wren, Self>> {
        if module != "<test>" || !is_static || classname != "Test" {
            return None;
        }
        return self.static_foreign.get(signature).copied();
    }
}

impl<'wren> UserData<'wren> {
    #[must_use]
    pub const fn get_output(&self) -> &String {
        &self.output
    }

    pub fn set_static_foreign_method(
        &mut self,
        signature: &'wren str,
        foreign: ForeignMethod<'wren, UserData<'wren>>,
    ) {
        self.static_foreign.insert(signature, foreign);
    }
}
#[macro_export]
macro_rules! call_test_case {
    (
        // What context to run these tests in
        // and also an optional type to case all return values
        // in this test to
        $vm:ident$(<$type:ty>)? {
            // Create zero or more test cases within a vm context
            $(
                // Match Class.method
                $class:ident.$method:ident
                // Check if there are parenthesis at all
                // could be
                // * Class.method
                // * Class.method()
                // * Class.method(1, 2, 3)
                $(
                    // If there are parenthesis then there are 0 or more args
                    // inside of them
                    ($($args:expr),*)
                )? == $res:expr
            )*
        }
    ) => {
        // We need this here since type is optional and it's not a repeated argument
        macro_rules! call_test_case_helper {
            ($c: expr, $h: expr, $a: expr) => {$vm.try_call$(::<$type, _>)?($c, $h, $a)}
        }
        $({
            let slice = wren_macros::to_signature!(
                // To signature takes one of
                // * method
                // * method()
                // * method(1,2,3)
                $method
                // Check if there is a paren
                $(
                    // add the paren and have a comma separated list of the args
                    ($($args),*)
                )?
            );
            let handle = $vm.make_call_handle_slice(slice).unwrap();
            // println!("{:?}, {}", handle, line!());
            let res = call_test_case_helper!(
                &$class,
                &handle,
                // Args should be
                // * &() if there is no argument list
                // * &() if there are zero arguments
                // * &(&1, &2, &3) if there are arguments
                //
                // First there should always be at least &()
                // So regardless of what happens on the inside
                // this should be here unconditionally
                &(
                    // Check if there was an argument list
                    $(
                        // Create a comma separated list of references to the args
                        $( & $args ),*
                    )?
                )
            );
            assert_eq!( res, $res );
        });*
    }
}

pub use call_test_case;

pub fn create_test_vm<'wren>(
    source: &str,
    fn_binding: impl FnOnce(&mut UserData<'wren>),
) -> (Vm<'wren, UserData<'wren>>, Handle<'wren>) {
    let mut vm = Vm::new(UserData::default());

    let vmptr = vm.get_context();
    fn_binding(vmptr.get_user_data_mut());
    vmptr
        .interpret("<test>", source)
        .expect("Code should run successfully");

    unsafe {
        vmptr.ensure_slots(1);
    }
    let class = vmptr
        .get_variable("<test>", "Test", 0)
        .expect("Test class should be defined in source");

    (vm, class)
}
