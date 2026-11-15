//! Namespace recognition, not origin inference or book write integration.
use super::*;

fn node() -> NodeId {
    "abcdefab-1234-5678-9abc-abcdefabcdef".parse().unwrap()
}

#[test]
fn generation_is_deterministic_and_parses_without_normalization() {
    let generated = ManagedLocalAlias::for_node(node());
    assert_eq!(
        generated.as_str(),
        "desktop-abcdefab-1234-5678-9abc-abcdefabcdef"
    );
    assert_eq!(ManagedLocalAlias::for_node(node()), generated);
    assert_eq!(
        ManagedLocalAlias::parse(generated.as_str()),
        Some(generated)
    );
}

#[test]
fn only_the_exact_generated_namespace_is_reserved() {
    for ordinary in [
        "desktop-pc",
        "desktop-home",
        "Desktop-abcdefab-1234-5678-9abc-abcdefabcdef",
        "desktop-ABCDEFAB-1234-5678-9ABC-ABCDEFABCDEF",
        "desktop-abcdefab123456789abcabcdefabcdef",
        "desktop-{abcdefab-1234-5678-9abc-abcdefabcdef}",
        "desktop-urn:uuid:abcdefab-1234-5678-9abc-abcdefabcdef",
        "desktop-abcdefab_1234-5678-9abc-abcdefabcdef",
        " desktop-abcdefab-1234-5678-9abc-abcdefabcdef",
        "desktop-abcdefab-1234-5678-9abc-abcdefabcdef ",
        "desktop-abcdefab-1234-5678-9abc-abcdefabcdef\n",
        "",
    ] {
        assert_eq!(ManagedLocalAlias::parse(ordinary), None, "{ordinary:?}");
    }
}

#[test]
fn recognition_is_bounded_before_uuid_decode_or_spelling_allocation() {
    let oversized = format!("desktop-{}", "a".repeat(4096));
    assert_eq!(ManagedLocalAlias::parse(&oversized), None);
    let unicode_same_character_count = format!("desktop-{}", "한".repeat(36));
    assert_eq!(
        ManagedLocalAlias::parse(&unicode_same_character_count),
        None
    );
    // A recognized legacy spelling still carries no Book/ManagedLocal origin.
    let legacy = "desktop-00000000-0000-0000-0000-000000000000";
    assert_eq!(ManagedLocalAlias::parse(legacy).unwrap().as_str(), legacy);
}
