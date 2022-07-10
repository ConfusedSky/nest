//! This is the simplest happy path

use wren::test::{call_test_case, create_test_vm};
use wren_macros::foreign_static_method;

#[foreign_static_method]
fn foreign_test(a: f64, b: f64, c: f64) -> f64 {
    a + b + c
}

#[foreign_static_method]
fn foreign_test2(a: String, b: String, c: String) -> String {
    a + &b + &c
}

fn main() {
    let (mut vm, test) = create_test_vm(
        "class Test {
        foreign static foreignTest(a, b, c)
        foreign static foreignTest2(a, b, c)
        static useForeignTest() { foreignTest(1, 2, 3) }
        static useForeignTest2() { foreignTest2(\"One\", \"Two\", \"Three\") }
    }",
        |f| {
            f.set_static_foreign_method("foreignTest(_,_,_)", foreign_test);
            f.set_static_foreign_method("foreignTest2(_,_,_)", foreign_test2);
        },
    );

    let context = vm.get_context();

    call_test_case!(context {
        test.useForeignTest() == Ok(6.0)
        test.useForeignTest2() == Ok("OneTwoThree".to_string())
    });
}
