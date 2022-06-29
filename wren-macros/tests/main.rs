#[test]
fn to_signature() {
    let t = trybuild::TestCases::new();
    t.pass("tests/to_signature.rs");
}
