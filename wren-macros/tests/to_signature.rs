use wren_macros::to_signature;

const TEST: &[u8] = to_signature!(testThing());
const TEST2: &[u8] = to_signature!(testThing);
const TEST3: &[u8] = to_signature!(testThing(1, 2, 3, 5));

fn main() {
    assert_eq!(TEST, b"testThing()\0");
    assert_eq!(TEST2, b"testThing\0");
    assert_eq!(TEST3, b"testThing(_,_,_,_)\0");
    assert_eq!(to_signature!(testThing(vec![1.0])), b"testThing(_)\0");
}
