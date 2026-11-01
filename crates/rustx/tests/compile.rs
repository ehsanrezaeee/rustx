#[test]
fn compile_diagnostics() {
    trybuild::TestCases::new().compile_fail("tests/ui/*.rs");
}
