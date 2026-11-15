use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Plugin(u8);

fn source() -> PanelCandidate<'static, Plugin> {
    PanelCandidate {
        identity: PanelIdentity::Builtin(BuiltinPanel::Source),
        selector: FileSelector::All,
        kind: Some(FileKind::Text),
    }
}
fn markdown() -> PanelCandidate<'static, Plugin> {
    PanelCandidate {
        identity: PanelIdentity::Builtin(BuiltinPanel::Markdown),
        selector: FileSelector::Names {
            extensions: &["md", "markdown"],
            filenames: &[],
        },
        kind: Some(FileKind::Text),
    }
}
fn plugin() -> PanelCandidate<'static, Plugin> {
    PanelCandidate {
        identity: PanelIdentity::Plugin(Plugin(1)),
        selector: FileSelector::Names {
            extensions: &["md", "rs"],
            filenames: &["Makefile"],
        },
        kind: None,
    }
}
fn path(text: &str) -> RelPath {
    text.parse().unwrap()
}

#[test]
fn rendering_builtin_suppresses_source_while_plugin_can_precede_the_floor() {
    let catalogue = [markdown(), plugin(), source()];
    let associations = BTreeMap::new();
    let md = resolve(
        &path("README.MD"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(
        md.selected.unwrap().identity,
        PanelIdentity::Builtin(BuiltinPanel::Markdown)
    );
    assert_eq!(md.candidates.len(), 2);
    assert!(md.asks_picker());
    assert_eq!(md.selector.kind(), SelectorKind::Extension);
    assert_eq!(md.selector.value(), "md");
    let rust = resolve(
        &path("file.rs"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(
        rust.selected.unwrap().identity,
        PanelIdentity::Plugin(Plugin(1))
    );
    assert_eq!(rust.candidates.len(), 2);
    let other = resolve(
        &path("file.txt"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(
        other.selected.unwrap().identity,
        PanelIdentity::Builtin(BuiltinPanel::Source)
    );
    assert!(!other.asks_picker());
}

#[test]
fn valid_tab_choice_precedes_association_and_uninstalled_choices_are_ignored() {
    let catalogue = [markdown(), plugin(), source()];
    let mut associations = BTreeMap::new();
    let key = resolve(
        &path("a.md"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap()
    .selector;
    let _old = associations.insert(key.clone(), PanelIdentity::Plugin(Plugin(1)));
    let associated = resolve(
        &path("b.md"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(
        associated.selected.unwrap().identity,
        PanelIdentity::Plugin(Plugin(1))
    );
    let choice = PanelIdentity::Builtin(BuiltinPanel::Markdown);
    let chosen = resolve(
        &path("b.md"),
        FileKind::Text,
        &catalogue,
        &associations,
        Some(&choice),
    )
    .unwrap();
    assert_eq!(chosen.selected.unwrap().identity, choice);
    let _old = associations.insert(key, PanelIdentity::Plugin(Plugin(9)));
    let unavailable = PanelIdentity::Plugin(Plugin(9));
    let fallback = resolve(
        &path("b.md"),
        FileKind::Text,
        &catalogue,
        &associations,
        Some(&unavailable),
    )
    .unwrap();
    assert_eq!(fallback.selected.unwrap().identity, choice);
    assert!(fallback.associated.is_none());
}

#[test]
fn filename_claims_are_case_sensitive_and_nontext_never_falls_back_to_source() {
    let catalogue = [markdown(), plugin(), source()];
    let associations = BTreeMap::new();
    let exact = resolve(
        &path("Makefile"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(exact.selector.kind(), SelectorKind::Filename);
    assert_eq!(exact.selector.value(), "Makefile");
    let other = resolve(
        &path("makefile"),
        FileKind::Text,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(other.selector.kind(), SelectorKind::Extension);
    let binary = resolve(
        &path("file.pdf"),
        FileKind::Binary,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert!(binary.selected.is_none());
    let claimed = resolve(
        &path("file.md"),
        FileKind::Binary,
        &catalogue,
        &associations,
        None,
    )
    .unwrap();
    assert_eq!(claimed.candidates.len(), 1);
    assert_eq!(
        claimed.selected.unwrap().identity,
        PanelIdentity::Plugin(Plugin(1))
    );
    assert!(!claimed.asks_picker());
}
