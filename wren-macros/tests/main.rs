#[test]
fn to_signature() {
    let t = trybuild::TestCases::new();
    t.pass("tests/to_signature.rs");
}

#[test]
fn to_foreign_method() {
    let t = trybuild::TestCases::new();
    t.pass("tests/to_foreign_method/*.success.rs");
    t.compile_fail("tests/to_foreign_method/*.failure.rs");
}
