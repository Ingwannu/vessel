//! A partial language is test data, never a shipped chooser entry. Read the
//! fixture root through the same resource constructor as the production path.
use super::{resolution::Bundle, runtime};

pub(super) fn source(tag: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/locales");
    std::fs::read_to_string(root.join(tag).join("meta.ftl")).unwrap()
}

pub(super) fn bundle(tag: &str) -> Bundle {
    runtime::fixture(tag, &source(tag)).unwrap()
}
