#[test]
pub fn test_vicocomo_bcrypt() {
    vicocomo::test_utils::test_crate(
        "../vicocomo/vicocomo_bcrypt",
        false,
        "test",
    );
}
