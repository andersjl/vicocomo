#[test]
pub fn test_vicocomo_bcrypt() {
    ljumvall_test_utils::test_crate(
        "../vicocomo/vicocomo_bcrypt",
        &["test"],
        false,
        false,
        ljumvall_test_utils::TestCrateOutput::None,
    );
}
