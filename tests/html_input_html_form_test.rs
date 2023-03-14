#[test]
fn test_html_html_form() {
    ljumvall_test_utils::test_crate(
        "../vicocomo/examples/html/html_form",
        &["run"],
        false,
        false,
        ljumvall_test_utils::TestCrateOutput::None,
    );
}
