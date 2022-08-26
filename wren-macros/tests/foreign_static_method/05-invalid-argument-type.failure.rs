//! This is the simplest happy path

use wren::test::{call_test_case, create_test_vm};
use wren_macros::foreign_static_method;

#[foreign_static_method]
fn test(a: f32, b: f32, c: f32) -> f32 {
    a + b + c
}

fn main() {
    let (mut vm, test) = create_test_vm(
        "class Test {
            foreign static foreignTest(a, b, c)
            static useForeignTest() { foreignTest(1, 2, 3) }
        }",
        |f| {
            f.set_static_foreign_method("foreignTest(_,_,_)", foreign_test);
        },
    );

    let context = vm.get_context();

    call_test_case!(context {
        test.useForeignTest() == Ok(6.0)
    });
}
