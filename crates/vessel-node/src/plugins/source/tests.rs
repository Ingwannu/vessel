use super::*;
use std::error::Error;

fn endpoint() -> RegistryEndpoint {
    RegistryEndpoint::parse("https://registry.example.invalid/base/").expect("registry")
}
fn subpath(path: &Option<RelPath>) -> Option<&str> {
    path.as_ref().map(RelPath::as_str)
}

#[test]
fn explicit_directory_shape_is_preserved_without_using_client_os_or_filesystem() {
    for raw in [
        ".",
        "..",
        "./owner/slug",
        "../owner/slug",
        "/owner/slug",
        "~/owner/slug",
        "~",
        r"C:\",
        r"D:\plugins\repository",
        r"\\server\share",
        r".\plugins\review",
    ] {
        let Source::Directory { input, subpath: _ } = parse(raw, None).expect("directory input")
        else {
            panic!("directory");
        };
        assert_eq!(input.0.as_str(), raw);
        let nested = format!("{raw}:plugins/main");
        let Source::Directory { input, subpath } =
            parse(&nested, None).expect("directory subpath input")
        else {
            panic!("directory");
        };
        assert_eq!(input.0.as_str(), raw);
        assert_eq!(subpath.as_ref().expect("subpath").as_str(), "plugins/main");
    }
}

#[test]
fn colon_suffix_does_not_confuse_uri_port_scp_separator_or_windows_drive() {
    for (raw, base, path) in [
        ("npm:review@1.2:nested", "review@1.2", Some("nested")),
        (
            "npm:@team/review@next:plugins/main",
            "@team/review@next",
            Some("plugins/main"),
        ),
        (
            "https://example.invalid:8443/repo.git",
            "https://example.invalid:8443/repo.git",
            None,
        ),
        (
            "https://example.invalid:8443/repo.git:plugins/main",
            "https://example.invalid:8443/repo.git",
            Some("plugins/main"),
        ),
        (
            "git:git@example.invalid:owner/repo.git",
            "git@example.invalid:owner/repo.git",
            None,
        ),
        (
            "git:git@example.invalid:owner/repo.git:plugins/main",
            "git@example.invalid:owner/repo.git",
            Some("plugins/main"),
        ),
        ("git@[::1]:owner/repo.git", "git@[::1]:owner/repo.git", None),
        (
            "git@[::1]:owner/repo.git:plugins/main",
            "git@[::1]:owner/repo.git",
            Some("plugins/main"),
        ),
        (
            "file:///D:/plugins/repository",
            "file:///D:/plugins/repository",
            None,
        ),
        (
            r"D:\plugins\repository:plugins/main",
            r"D:\plugins\repository",
            Some("plugins/main"),
        ),
    ] {
        let parsed = reference(raw).expect("reference");
        assert_eq!(parsed.source.as_str(), base);
        assert_eq!(subpath(&parsed.subpath), path);
    }
}

#[test]
fn default_and_host_port_registries_win_before_git_or_npm_guessing() {
    let configured = endpoint();
    let Source::Registry(source) = parse("owner/slug", Some(&configured)).expect("registry") else {
        panic!("registry");
    };
    assert_eq!(source.id.as_str(), "owner/slug");
    assert_eq!(source.endpoint, configured);
    let Source::Registry(source) =
        parse("private.example.invalid:8443/owner/slug", None).expect("explicit")
    else {
        panic!("explicit registry");
    };
    assert_eq!(
        source.endpoint.as_str(),
        "https://private.example.invalid:8443/"
    );
    assert!(matches!(
        parse("owner/slug", None),
        Err(SourceError::NoRegistry)
    ));
    assert!(matches!(
        parse("owner/slug:plugins/main", Some(&configured)),
        Err(SourceError::RegistryOverride)
    ));
    let Source::Git { input, subpath } =
        parse("github:owner/repository:plugins/main", Some(&configured))
            .expect("explicit Git input")
    else {
        panic!("Git input");
    };
    assert_eq!(input.prefix, Prefix::Github);
    assert_eq!(input.source.as_str(), "owner/repository");
    assert_eq!(subpath.as_ref().expect("subpath").as_str(), "plugins/main");
}

#[test]
fn npm_package_and_selector_grammar_is_literal_and_bounded() {
    let Source::Npm { input, subpath } =
        parse("npm:@team/plugin@>=1.2 <2:nested", None).expect("npm")
    else {
        panic!("npm");
    };
    assert_eq!(input.package.as_str(), "@team/plugin");
    assert_eq!(input.selector.as_str(), ">=1.2 <2");
    assert_eq!(subpath.as_ref().expect("path").as_str(), "nested");
    let Source::Npm { input, .. } = parse("simple-package", None).expect("default npm") else {
        panic!("npm");
    };
    assert_eq!(input.selector.as_str(), "latest");
    for invalid in [
        "npm:",
        "npm:@scope/",
        "npm:UPPER",
        "npm:pkg@",
        "npm:pkg@ next",
        "npm:pkg@tag/evil",
        "npm:pkg@tag@evil",
    ] {
        assert!(parse(invalid, None).is_err(), "invalid npm input");
    }
    assert!(npm(&"x".repeat(reference::MAX_SOURCE_BYTES.saturating_add(1))).is_err());
}

fn safe(error: &dyn Error) {
    let mut cause = error;
    loop {
        assert!(!format!("{cause:?} {cause}").contains("PRIVATE-SOURCE-CANARY"));
        match cause.source() {
            Some(next) => cause = next,
            None => break,
        }
    }
}
#[test]
fn origin_validation_and_diagnostics_never_disclose_credentials_or_private_paths() {
    for invalid in [
        "https://PRIVATE-SOURCE-CANARY@example.invalid",
        "https://[PRIVATE-SOURCE-CANARY]/",
        "https://example.invalid/?PRIVATE-SOURCE-CANARY",
        "http://public.example.invalid/PRIVATE-SOURCE-CANARY",
    ] {
        safe(&RegistryEndpoint::parse(invalid).expect_err("unsafe origin"));
    }
    for local in [
        "http://localhost:8123/base",
        "http://127.0.0.1:8123/base",
        "http://[::1]:8123/base",
    ] {
        assert!(RegistryEndpoint::parse(local).is_ok());
    }
    for input in [
        "/PRIVATE-SOURCE-CANARY",
        "git:ssh://git@example.invalid/PRIVATE-SOURCE-CANARY",
        "npm:PRIVATE-SOURCE-CANARY",
    ] {
        match parse(input, None) {
            Ok(source) => assert!(!format!("{source:?}").contains("PRIVATE-SOURCE-CANARY")),
            Err(error) => safe(&error),
        }
    }
}
