//! This is the simplest happy path

use wren::context::Foreign;
use wren::test::{call_test_case, create_test_vm, Context};
use wren_macros::foreign_static_method;

#[foreign_static_method]
fn test(a: f64, b: f64, c: f64) -> f64 {
    a + b + c
}

#[foreign_static_method]
fn test2(a: String, b: String, c: String) -> String {
    a + &b + &c
}

#[foreign_static_method]
fn test3(context: &mut Context<'_, Foreign>, a: f64) -> f64 {
    assert!(context.get_user_data_mut().get_output().is_empty());

    a
}

#[foreign_static_method]
fn test4<'a>() -> Result<&'a str, &'a str> {
    Err("This thing failed or something")
}

#[foreign_static_method]
fn test5<'a>() -> &'a str {
    "This is a test str, since this was failing as well"
}

fn main() {
    let (mut vm, test) = create_test_vm(
        "class Test {
        foreign static foreignTest(a, b, c)
        foreign static foreignTest2(a, b, c)
        foreign static foreignTest3(a)
        foreign static foreignTest4()
        static useForeignTest() { foreignTest(1, 2, 3) }
        static useForeignTest2() { foreignTest2(\"One\", \"Two\", \"Three\") }
        static useForeignTest4() {
            var result = Fiber.new {
                foreignTest4()
            }.try()

            return result
        }
    }",
        |f| {
            f.set_static_foreign_method("foreignTest(_,_,_)", foreign_test);
            f.set_static_foreign_method("foreignTest2(_,_,_)", foreign_test2);
            f.set_static_foreign_method("foreignTest3(_)", foreign_test3);
            f.set_static_foreign_method("foreignTest4()", foreign_test4);
        },
    );

    let context = vm.get_context();

    call_test_case!(context {
        test.useForeignTest() == Ok(6.0)
        test.useForeignTest2() == Ok("OneTwoThree".to_string())
        test.foreignTest3(1.0) == Ok(1.0)
        test.useForeignTest4() == Ok("This thing failed or something".to_string())
    });
}
