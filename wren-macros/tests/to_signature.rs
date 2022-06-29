use wren_macros::to_signature;

const TEST: &str = to_signature!(testThing());
const TEST2: &str = to_signature!(testThing);
const TEST3: &str = to_signature!(testThing(1, 2, 3, 5));

fn main() {
    assert_eq!(TEST, "testThing()");
    assert_eq!(TEST2, "testThing");
    assert_eq!(TEST3, "testThing(_,_,_,_)");
    assert_eq!(to_signature!(testThing(vec![1.0])), "testThing(_)");
}
