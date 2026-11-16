use super::super::{
    Appearance, Contribution, ContributionContent, Control, ControlContent, RuntimeGeneration,
    Surface, ThemeColors, ThemeContribution,
};
use super::*;
use std::error::Error;

fn installation(node: NodeId, id: &str, document: ClientDocument) -> ClientInstallation {
    ClientInstallation::new(
        PluginAddress {
            node,
            plugin: id.parse().expect("id"),
            generation: RuntimeGeneration::random(),
        },
        ProtocolVersion::new(23),
        document,
    )
    .expect("installation")
}
fn empty() -> ClientDocument {
    ClientDocument::new(vec![], vec![], vec![]).expect("empty")
}

#[test]
fn immutable_catalog_roundtrips_with_exact_owner_generation_and_cached_size() {
    let node = NodeId::random();
    let item = installation(node, "one", empty());
    let address = item.address().clone();
    assert_eq!(item.required_client(), ProtocolVersion::new(23));
    assert!(item.document().contributions().is_empty());
    let catalog = ClientCatalog::new(node, vec![item]).expect("catalog");
    let bytes = serde_json::to_vec(&catalog).expect("encode");
    assert_eq!(catalog.encoded_bytes(), bytes.len());
    assert!(!String::from_utf8_lossy(&bytes).contains("encoded_bytes"));
    let decoded = ClientCatalog::decode(&bytes).expect("decode");
    assert_eq!(decoded, catalog);
    assert_eq!(decoded.node(), node);
    let [item] = decoded.installations() else {
        panic!("one installation");
    };
    assert_eq!(item.address(), &address);
}

#[test]
fn owner_duplicates_and_zero_requirements_refuse_before_publication() {
    let node = NodeId::random();
    assert!(matches!(
        ClientCatalog::new(NodeId::random(), vec![installation(node, "one", empty())]),
        Err(CatalogError::Owner)
    ));
    assert!(matches!(
        ClientCatalog::new(
            node,
            vec![
                installation(node, "one", empty()),
                installation(node, "one", empty())
            ]
        ),
        Err(CatalogError::Duplicate)
    ));
    let address = installation(node, "one", empty()).address().clone();
    assert!(matches!(
        ClientInstallation::new(address, ProtocolVersion::new(0), empty()),
        Err(CatalogError::Requirement)
    ));
    let items = (0..MAX_CATALOG_INSTALLATIONS.saturating_add(1))
        .map(|i| installation(node, &format!("plugin-{i}"), empty()))
        .collect();
    assert!(matches!(
        ClientCatalog::new(node, items),
        Err(CatalogError::Installations)
    ));
}

fn themes(count: usize) -> ClientDocument {
    let color: super::super::HexColor = "#123".parse().expect("color");
    let colors = ThemeColors {
        background: color.clone(),
        foreground: color.clone(),
        raised: color.clone(),
        control: color.clone(),
        border: color.clone(),
        accent: None,
        muted_foreground: color.clone(),
        ring: color,
    };
    let themes = (0..count)
        .map(|i| ThemeContribution {
            id: format!("theme-{i}").parse().expect("id"),
            name: "Theme".parse().expect("name"),
            appearance: Appearance::Dark,
            colors: colors.clone(),
        })
        .collect();
    ClientDocument::new(vec![], vec![], themes).expect("themes")
}

#[test]
fn theme_admission_is_whole_node_not_multiplied_per_installation() {
    let node = NodeId::random();
    let first = installation(node, "one", themes(MAX_CATALOG_THEMES));
    let second = installation(node, "two", themes(1));
    assert!(matches!(
        ClientCatalog::new(node, vec![first, second]),
        Err(CatalogError::Themes)
    ));
}

fn large_document() -> ClientDocument {
    let text: super::super::ContentText = "x".repeat(16 * 1024).parse().expect("bounded content");
    let controls = (0..60)
        .map(|i| Control {
            id: format!("control-{i}").parse().expect("id"),
            content: ControlContent::Text { text: text.clone() },
        })
        .collect();
    ClientDocument::new(
        vec![],
        vec![Contribution {
            id: "screen".parse().expect("id"),
            title: "Screen".parse().expect("title"),
            content: ContributionContent::Screen {
                surface: Surface { controls },
            },
        }],
        vec![],
    )
    .expect("bounded document")
}

#[test]
fn whole_catalog_byte_admission_also_applies_to_typed_construction() {
    let node = NodeId::random();
    let document = large_document();
    let items = (0..5)
        .map(|i| installation(node, &format!("plugin-{i}"), document.clone()))
        .collect();
    assert!(matches!(
        ClientCatalog::new(node, items),
        Err(CatalogError::Bytes)
    ));
    assert!(matches!(
        ClientCatalog::decode(&vec![b' '; MAX_CATALOG_BYTES.saturating_add(1)]),
        Err(CatalogError::Bytes)
    ));
}

fn safe(error: &dyn Error) {
    let mut cause = error;
    loop {
        assert!(!format!("{cause:?} {cause}").contains("PRIVATE-CATALOG-CANARY"));
        match cause.source() {
            Some(next) => cause = next,
            None => break,
        }
    }
}

#[test]
fn direct_generic_and_nested_installation_decoders_seal_all_diagnostics() {
    let node = NodeId::random();
    let item = installation(node, "one", empty());
    let item_json = serde_json::to_string(&item).expect("item JSON");
    for invalid in [
        item_json.replace("required_client", "PRIVATE-CATALOG-CANARY"),
        item_json.replace(
            "\"required_client\":23",
            "\"required_client\":\"PRIVATE-CATALOG-CANARY\"",
        ),
    ] {
        safe(&serde_json::from_str::<ClientInstallation>(&invalid).expect_err("refuse"));
        let envelope = format!(r#"{{"node":"{node}","installations":[{invalid}]}}"#);
        safe(&ClientCatalog::decode(envelope.as_bytes()).expect_err("refuse"));
        safe(&serde_json::from_str::<ClientCatalog>(&envelope).expect_err("refuse"));
    }
    let outer = format!(r#"{{"node":"{node}","installations":[],"PRIVATE-CATALOG-CANARY":true}}"#);
    safe(&ClientCatalog::decode(outer.as_bytes()).expect_err("refuse"));
    safe(&serde_json::from_str::<ClientCatalog>(&outer).expect_err("refuse"));
}
