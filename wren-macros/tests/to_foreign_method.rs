use wren::test::create_test_vm;
use wren_macros::foreign_static_method;

#[foreign_static_method]
fn foreign_test(a: f32, b: f32, c: f32) -> f32 {
    a + b + c
}

fn main() {
    let (vm, test) = create_test_vm(
        "class Test {
        foreign static foreignTest(a, b, c)
        static useForeignTest() { foreignTest(1, 2, 3) }
    }",
        |f| {
            f.set_static_foreign_method("foreign_test(_,_,_)", foreign_test);
        },
    );
}
