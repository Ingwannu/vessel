//! Portable path fixtures are data: they never resolve a share or start a program.

use super::*;

#[test]
fn every_documented_windows_prefix_and_unix_path_survives_another_host() {
    for (text, syntax) in [
        ("/opt/tools/port", ExecPathSyntax::Unix),
        (r"C:\tools\port.exe", ExecPathSyntax::Windows),
        ("c:/tools/port.exe", ExecPathSyntax::Windows),
        (r"\\host\share\port.exe", ExecPathSyntax::Windows),
        ("//host/share/port.exe", ExecPathSyntax::Windows),
        (r"\\?\C:\tools\port.exe", ExecPathSyntax::Windows),
        (r"\\?\UNC\host\share\port.exe", ExecPathSyntax::Windows),
        (r"\\?\UNC/host\share\port.exe", ExecPathSyntax::Windows),
        (r"\\?\volume\tools\port.exe", ExecPathSyntax::Windows),
        (r"\\.\C:\tools\port.exe", ExecPathSyntax::Windows),
        ("//./C:/tools/port.exe", ExecPathSyntax::Windows),
        (r"/\host\share\port.exe", ExecPathSyntax::Either),
    ] {
        let path: ExecPath = text.parse().unwrap();
        assert_eq!(path.as_str(), text);
        assert_eq!(path.syntax(), syntax, "{text:?}");
        let json = serde_json::to_string(&path).unwrap();
        assert_eq!(serde_json::from_str::<ExecPath>(&json).unwrap(), path);
    }
}

#[test]
fn raw_empty_dot_and_parent_parts_are_refused_before_normalization() {
    for text in [
        "",
        "/",
        "/a//b",
        "/a/./b",
        "/a/../b",
        "/a/b/",
        "a/b",
        r"C:tools\port.exe",
        r"\tools\port.exe",
        r"C:\",
        r"C:\a\\b",
        r"C:\a\.\b",
        r"C:\a\..\b",
        r"\\host\share",
        r"\\host\share\",
        r"\\host\\share\port.exe",
        r"\\host\share\..\port.exe",
        r"\\?\C:/tools\port.exe",
        r"\\?\C:\a\.\port.exe",
        r"\\?\UNC\host\share\..\port.exe",
        r"\\?\UNC/host\share",
        r"\\?\UNC/host\\port.exe",
        r"\\.\device",
        "/a\0b",
    ] {
        assert!(text.parse::<ExecPath>().is_err(), "{text:?}");
        let json = serde_json::to_string(text).unwrap();
        assert!(serde_json::from_str::<ExecPath>(&json).is_err(), "{text:?}");
    }
}

#[test]
fn literal_backslashes_and_verbatim_separators_are_not_reinterpreted() {
    let unix: ExecPath = r"/tmp/name\..\program".parse().unwrap();
    assert_eq!(unix.syntax(), ExecPathSyntax::Unix);
    let mixed: ExecPath = r"/\host\share\..\program".parse().unwrap();
    assert_eq!(mixed.syntax(), ExecPathSyntax::Unix);
    let literal: ExecPath = r"\\?\C:\directory/with/slashes\program".parse().unwrap();
    assert_eq!(literal.syntax(), ExecPathSyntax::Windows);
    assert_eq!(literal.as_str(), r"\\?\C:\directory/with/slashes\program");
}

#[test]
fn control_characters_cannot_redraw_the_approved_path() {
    for control in ['\n', '\r', '\u{1b}', '\u{7}', '\u{7f}', '\u{85}'] {
        for prefix in ["/tools/", r"C:\tools\", r"\\host\share\"] {
            let text = format!("{prefix}port{control}name");
            assert_eq!(
                text.parse::<ExecPath>().unwrap_err(),
                InvalidExecPath::Control,
                "{text:?}"
            );
            let json = serde_json::to_string(&text).unwrap();
            assert!(serde_json::from_str::<ExecPath>(&json).is_err(), "{text:?}");
        }
    }
    assert_eq!(
        "/tools/port\0name".parse::<ExecPath>().unwrap_err(),
        InvalidExecPath::Nul
    );
}

#[test]
fn the_shared_byte_bound_applies_to_both_text_and_serde() {
    let root = "/";
    let text = format!("{root}{}", "x".repeat(ExecPath::MAX_BYTES - root.len()));
    let path: ExecPath = text.parse().unwrap();
    assert_eq!(path.as_str(), text);
    let too_long = format!("{text}x");
    assert_eq!(
        too_long.parse::<ExecPath>().unwrap_err(),
        InvalidExecPath::TooLong
    );
    let json = serde_json::to_string(&too_long).unwrap();
    assert!(serde_json::from_str::<ExecPath>(&json).is_err());
    let multibyte = format!(
        "{root}{}",
        "é".repeat(ExecPath::MAX_BYTES.div_ceil("é".len()))
    );
    assert_eq!(
        multibyte.parse::<ExecPath>().unwrap_err(),
        InvalidExecPath::TooLong
    );
}
