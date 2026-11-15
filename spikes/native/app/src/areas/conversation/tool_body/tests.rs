#[test]
fn borrowed_request_sections_preserve_structural_parent_identity() {
    use vessel_app_kit::ui::ElementId;
    let request = ElementId::NamedInteger("permit".into(), 1);
    let text_alias = ElementId::Name("permit-1".into());
    assert_eq!(request.to_string(), text_alias.to_string());
    assert_ne!(
        super::child_id(&request, "section-0"),
        super::child_id(&text_alias, "section-0")
    );
    assert_ne!(
        super::child_id(&request, "section-0"),
        super::child_id(&request, "failure")
    );
    assert_eq!(
        super::child_id(&request, "section-0"),
        super::child_id(&request, "section-0")
    );
}
