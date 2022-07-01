use wren_macros::{call_signature, to_signature};

const TEST: &[u8] = to_signature!(testThing());
const TEST2: &[u8] = to_signature!(testThing);
const TEST3: &[u8] = to_signature!(testThing(1, 2, 3, 5));
const TEST4: &[u8] = to_signature!(testThing(vec![1.0]));

const TEST5: &[u8] = call_signature!(testThing, 0);
const TEST6: &[u8] = call_signature!(testThing);
const TEST7: &[u8] = call_signature!(testThing, 4);
const TEST8: &[u8] = call_signature!(testThing, 1);

fn main() {
    assert_eq!(TEST, b"testThing()\0");
    assert_eq!(TEST2, b"testThing\0");
    assert_eq!(TEST3, b"testThing(_,_,_,_)\0");
    assert_eq!(TEST4, b"testThing(_)\0");
    assert_eq!(TEST5, b"testThing()\0");
    assert_eq!(TEST6, b"testThing\0");
    assert_eq!(TEST7, b"testThing(_,_,_,_)\0");
    assert_eq!(TEST8, b"testThing(_)\0");
}
