use super::*;
use std::error::Error;

const STATIC: &str = r#"{"format":"vessel.plugin/v1","id":"sample","name":"Sample","description":"Sample package","icon":"icon.PNG","media":["preview.png","https://example.invalid/preview.png"],"requirements":{"node":23,"client":23},"client":"client.json"}"#;

fn decode(source: &str) -> Result<Manifest, ManifestError> {
    Manifest::decode(source.as_bytes())
}

#[test]
fn static_package_requires_both_compatible_protocol_floors() {
    let manifest = decode(STATIC).expect("static native data");
    assert_eq!(manifest.0.id.as_str(), "sample");
    assert_eq!(manifest.0.name.as_ref().expect("name").as_str(), "Sample");
    assert_eq!(
        manifest
            .0
            .description
            .as_ref()
            .expect("description")
            .as_str(),
        "Sample package"
    );
    assert_eq!(
        manifest.0.client.as_ref().expect("client").as_str(),
        "client.json"
    );
    let [Media::Package(path), Media::Https(url)] = manifest.0.media.as_slice() else {
        panic!("media");
    };
    assert_eq!(path.as_str(), "preview.png");
    assert_eq!(url.as_str(), "https://example.invalid/preview.png");
    let current = ProtocolVersion::new(23);
    manifest.compatible(current, current).expect("compatible");
    assert!(matches!(
        manifest.compatible(ProtocolVersion::new(22), current),
        Err(ManifestError::Incompatible)
    ));
    assert!(matches!(
        manifest.compatible(current, ProtocolVersion::new(22)),
        Err(ManifestError::Incompatible)
    ));
    assert!(matches!(
        manifest.helper(Target::LinuxX64),
        Err(ManifestError::UnsupportedTarget)
    ));
}

#[test]
fn exact_target_selection_never_substitutes_another_architecture_or_libc() {
    let manifest = decode(r#"{"format":"vessel.plugin/v1","id":"helper","requirements":{"node":23,"client":23},"helpers":[{"target":"x86_64-unknown-linux-gnu","protocol":1,"program":"bin/helper","args":["--stdio"]}],"build":[["tool","build"]]}"#).expect("helper");
    let helper = manifest.helper(Target::LinuxX64).expect("exact");
    assert_eq!(helper.program.as_str(), "bin/helper");
    assert_eq!(helper.args, ["--stdio"]);
    assert!(matches!(
        manifest.helper(Target::LinuxMuslX64),
        Err(ManifestError::UnsupportedTarget)
    ));
    assert_eq!(manifest.0.build, [vec!["tool", "build"]]);
    for target in [
        Target::LinuxArm64,
        Target::LinuxMuslArm64,
        Target::MacX64,
        Target::MacArm64,
        Target::WindowsX64,
        Target::WindowsArm64,
    ] {
        assert!(matches!(
            manifest.helper(target),
            Err(ManifestError::UnsupportedTarget)
        ));
    }
}

#[test]
fn unknown_format_fields_targets_and_incomplete_or_empty_declarations_refuse() {
    for invalid in [
        STATIC.replace("vessel.plugin/v1", "vessel.plugin/v2"),
        STATIC.replace("\"client\":23", "\"paseo\":\">=0.8\""),
        STATIC.replace("\"node\":23", "\"node\":0"),
        STATIC.replace("icon.PNG", "icon.svg"),
        STATIC.replace(
            "https://example.invalid/preview.png",
            "http://example.invalid/preview.png",
        ),
        STATIC.replace(
            "https://example.invalid/preview.png",
            "https://private@example.invalid/preview.png",
        ),
        STATIC.replace(
            "https://example.invalid/preview.png",
            "https://[bad]/preview.png",
        ),
        STATIC.replace("\"client\":\"client.json\"", "\"client\":null"),
    ] {
        assert!(decode(&invalid).is_err());
    }
    assert!(
        decode(&STATIC.replace(
            "\"client\":\"client.json\"",
            "\"client\":\"client.json\",\"build\":[[]]"
        ))
        .is_err()
    );
}

#[test]
fn paths_refuse_traversal_drives_streams_and_js_edge_whitespace() {
    for raw in [
        "",
        "../outside",
        "/outside",
        "a/../outside",
        "C:outside",
        "x:stream",
        "a\\b",
        "a//b",
        "a/",
        "a?query",
        "a#fragment",
        "\u{feff}client.json",
        "client.json\u{feff}",
    ] {
        assert!(
            PackagePath::try_from(raw.to_owned()).is_err(),
            "invalid package path"
        );
    }
    assert_eq!(
        PackagePath::try_from("문서/client file.json".to_owned())
            .expect("literal")
            .as_str(),
        "문서/client file.json"
    );
}

#[test]
fn parser_errors_and_private_manifest_debug_seal_input_at_every_source_level() {
    for source in [
        STATIC.replace("client.json", "PRIVATE-MANIFEST-CANARY"),
        STATIC.replace("vessel.plugin/v1", "PRIVATE-MANIFEST-CANARY"),
    ] {
        match decode(&source) {
            Ok(manifest) => assert!(!format!("{manifest:?}").contains("PRIVATE-MANIFEST-CANARY")),
            Err(error) => {
                let mut cause: &dyn Error = &error;
                loop {
                    assert!(!format!("{cause:?} {cause}").contains("PRIVATE-MANIFEST-CANARY"));
                    match cause.source() {
                        Some(next) => cause = next,
                        None => break,
                    }
                }
            }
        }
    }
    assert!(
        !format!(
            "{:?}",
            PackagePath::try_from("PRIVATE-MANIFEST-CANARY".to_owned()).expect("path")
        )
        .contains("PRIVATE-MANIFEST-CANARY")
    );
}

#[test]
fn manifest_and_argv_budgets_refuse_whole_input_without_truncating() {
    assert!(matches!(
        Manifest::decode(&vec![b' '; MAX_MANIFEST_BYTES.saturating_add(1)]),
        Err(ManifestError::Bytes)
    ));
    assert!(matches!(
        arguments(&vec!["x".to_owned(); MAX_ARGUMENTS.saturating_add(1)]),
        Err(ManifestError::Capacity)
    ));
    assert!(matches!(
        arguments(&["x".repeat(MAX_ARGUMENT_BYTES.saturating_add(1))]),
        Err(ManifestError::Capacity)
    ));
    assert!(matches!(
        arguments(&vec!["x".repeat(MAX_ARGUMENT_BYTES); 3]),
        Err(ManifestError::Capacity)
    ));
    assert!(matches!(
        arguments(&["\u{feff}".to_owned()]),
        Err(ManifestError::Argument)
    ));
    assert!(matches!(
        arguments(&["a\0b".to_owned()]),
        Err(ManifestError::Argument)
    ));
    let mut manifest = decode(STATIC).expect("static");
    manifest.0.media = (0..MAX_MEDIA.saturating_add(1))
        .map(|_| Media::try_from("preview.png".to_owned()).expect("path"))
        .collect();
    assert!(matches!(manifest.validate(), Err(ManifestError::Capacity)));
}

#[test]
fn duplicate_and_unknown_helper_targets_refuse_the_whole_manifest() {
    let helper = r#"{"target":"x86_64-unknown-linux-gnu","protocol":1,"program":"bin/helper"}"#;
    let base = r#"{"format":"vessel.plugin/v1","id":"helper","requirements":{"node":23,"client":23},"helpers":HELPERS}"#;
    let duplicate = base.replace("HELPERS", &format!("[{helper},{helper}]"));
    assert!(matches!(
        decode(&duplicate),
        Err(ManifestError::DuplicateTarget)
    ));
    let unknown = base.replace(
        "HELPERS",
        &format!(
            "[{}]",
            helper.replace("x86_64-unknown-linux-gnu", "unknown-target")
        ),
    );
    assert!(matches!(decode(&unknown), Err(ManifestError::Decode(_))));
    let unknown_protocol = base.replace(
        "HELPERS",
        &format!("[{}]", helper.replace("\"protocol\":1", "\"protocol\":2")),
    );
    assert!(matches!(
        decode(&unknown_protocol),
        Err(ManifestError::Decode(_))
    ));
}

#[test]
fn broader_author_targets_never_imply_release_eligibility() {
    assert!(Target::LinuxX64.eligible());
    assert!(Target::WindowsX64.eligible());
    for target in [
        Target::LinuxArm64,
        Target::LinuxMuslX64,
        Target::LinuxMuslArm64,
        Target::MacX64,
        Target::MacArm64,
        Target::WindowsArm64,
    ] {
        assert!(!target.eligible());
    }
    let helper = r#"{"target":"x86_64-unknown-linux-musl","protocol":1,"program":"bin/helper"}"#;
    let source = format!(
        r#"{{"format":"vessel.plugin/v1","id":"helper","requirements":{{"node":23,"client":23}},"helpers":[{helper}]}}"#
    );
    let manifest = decode(&source).expect("author schema data");
    assert!(matches!(
        manifest.helper(Target::LinuxMuslX64),
        Err(ManifestError::UnsupportedTarget)
    ));
    match env!("VESSEL_TARGET") {
        "x86_64-unknown-linux-gnu" => assert_eq!(
            Target::compiled().expect("compiled GNU target"),
            Target::LinuxX64
        ),
        "x86_64-pc-windows-msvc" => assert_eq!(
            Target::compiled().expect("compiled MSVC target"),
            Target::WindowsX64
        ),
        _ => assert!(matches!(
            Target::compiled(),
            Err(ManifestError::UnsupportedTarget)
        )),
    }
}
