#[test]
fn to_signature() {
    let t = trybuild::TestCases::new();
    t.pass("tests/to_signature.rs");
}

#[test]
fn foreign_static_method() {
    let t = trybuild::TestCases::new();
    t.pass("tests/foreign_static_method/*.success.rs");
    t.compile_fail("tests/foreign_static_method/*.failure.rs");
}
