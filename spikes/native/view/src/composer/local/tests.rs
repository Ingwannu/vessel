use super::*;
#[test]
fn persisted_keys_and_lengths_cannot_bypass_their_validators() {
    for value in [
        "", ".", "..", "../x", "x/y", "x\\y", "x.bin", "a b", "a\n", "한글",
    ] {
        assert!(LocalKey::parse(value).is_err());
        assert!(serde_json::from_str::<LocalKey>(&serde_json::to_string(value).unwrap()).is_err());
    }
    assert!(LocalKey::parse(&"x".repeat(LocalKey::MAX.saturating_add(1))).is_err());
    assert!(LocalKey::parse("attachment_ABC-123").is_ok());
    assert!(LocalSize::new(0).is_err());
    assert!(LocalSize::new(LocalSize::MAX.saturating_add(1)).is_err());
    assert!(serde_json::from_str::<LocalSize>("0").is_err());
    assert_eq!(
        LocalSize::new(LocalSize::MAX).unwrap().value(),
        LocalSize::MAX
    );
}
