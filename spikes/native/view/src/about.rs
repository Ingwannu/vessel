//! What Settings › About lists: the software the app is built from and the
//! licence each is under, as the page's (`web/src/data/about.ts`). A list
//! kept by hand, checked against the workspace's `Cargo.toml` by its test,
//! so a dependency added without a line here fails `cargo test` rather
//! than going unlisted (the page's test checks `package.json`).
//!
//! [Decision Log]
//! - Purpose: the page's licences, for the app's own parts.
//! - Choice: the workspace's dependencies by their published names, in
//!   the manifest's order, then the faces the kit bundles; each licence as
//!   its crate declares it (`cargo metadata`), the faces' from their OFL
//!   texts beside them.

/// What a part does in the app (its words are in `text/settings.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The windows' drawing.
    Drawing,
    /// The platform's windows.
    Windows,
    /// Text editing.
    Editing,
    /// Controls.
    Controls,
    /// Bytes on the wire.
    Bytes,
    /// Dates and times.
    Dates,
    /// The platform's folders.
    Folders,
    /// This computer's name.
    ComputerName,
    /// Reading and writing data.
    Data,
    /// The system's language.
    Language,
    /// Errors.
    Errors,
    /// The network and its tasks.
    Network,
    /// Logs.
    Logs,
    /// Find's folding of accents.
    Accents,
    /// The face of everything.
    Face,
    /// The face of code.
    CodeFace,
    /// The face of Hangul.
    Hangul,
}

/// One part the app bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bundled {
    /// Its published name.
    pub name: &'static str,
    /// Its SPDX licence expression.
    pub licence: &'static str,
    /// What it does here.
    pub role: Role,
    /// A face's (not a crate's).
    pub face: bool,
}

/// A crate.
const fn krate(name: &'static str, licence: &'static str, role: Role) -> Bundled {
    Bundled {
        name,
        licence,
        role,
        face: false,
    }
}

/// A face.
const fn face(name: &'static str, role: Role) -> Bundled {
    Bundled {
        name,
        licence: "OFL-1.1",
        role,
        face: true,
    }
}

/// Every dependency of the workspace, in its manifest's order, then the
/// faces.
pub const BUNDLED: [Bundled; 19] = [
    krate("gpui-pre", "Apache-2.0", Role::Drawing),
    krate("gpui-pre-platform", "Apache-2.0", Role::Windows),
    krate("gpui-component", "Apache-2.0", Role::Controls),
    krate("gpui-base", "Apache-2.0", Role::Editing),
    krate("bytes", "MIT", Role::Bytes),
    krate("chrono", "MIT OR Apache-2.0", Role::Dates),
    krate("dirs", "MIT OR Apache-2.0", Role::Folders),
    krate("hostname", "MIT", Role::ComputerName),
    krate("serde", "MIT OR Apache-2.0", Role::Data),
    krate("serde_json", "MIT OR Apache-2.0", Role::Data),
    krate("sys-locale", "MIT OR Apache-2.0", Role::Language),
    krate("thiserror", "MIT OR Apache-2.0", Role::Errors),
    krate("tokio", "MIT", Role::Network),
    krate("tracing", "MIT", Role::Logs),
    krate("tracing-subscriber", "MIT", Role::Logs),
    krate(
        "unicode-normalization",
        "(MIT OR Apache-2.0) AND Unicode-3.0",
        Role::Accents,
    ),
    face("Geist", Role::Face),
    face("Geist Mono", Role::CodeFace),
    face("Pretendard", Role::Hangul),
];

/// Vessel's own licence (`LICENSE.md`, ADR 0014).
pub const VESSEL_LICENCE: &str = "PolyForm Strict 1.0.0";

#[cfg(test)]
mod tests {
    use super::BUNDLED;

    /// The published names of the workspace's dependencies that are not
    /// Vessel's own, in the manifest's order.
    fn dependencies(manifest: &str) -> Vec<String> {
        let mut inside = false;
        let mut names = Vec::new();
        for line in manifest.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                inside = line == "[workspace.dependencies]";
                continue;
            }
            if !inside || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once(" = ") else {
                continue;
            };
            let name = value
                .split_once("package = \"")
                .and_then(|(_, rest)| rest.split_once('"'))
                .map_or(key, |(package, _)| package);
            if !name.starts_with("vessel-") {
                names.push(name.to_owned());
            }
        }
        names
    }

    #[test]
    fn name_every_dependency_in_the_manifests_order() {
        let listed: Vec<&str> = BUNDLED
            .iter()
            .filter(|entry| !entry.face)
            .map(|entry| entry.name)
            .collect();
        assert_eq!(
            listed,
            dependencies(include_str!("../../Cargo.toml")),
            "a dependency of spikes/native/Cargo.toml is missing from BUNDLED, or out of order"
        );
    }

    #[test]
    fn list_every_face_the_kit_bundles() {
        let fonts = include_str!("../../kit/src/fonts.rs");
        for (face, file) in [
            ("Geist", "Geist-Regular"),
            ("Geist Mono", "GeistMono-Regular"),
            ("Pretendard", "Pretendard-Regular"),
        ] {
            assert!(fonts.contains(file), "{file} is no longer bundled");
            assert!(
                BUNDLED.iter().any(|entry| entry.face && entry.name == face),
                "{face} is bundled and not listed"
            );
        }
    }
}
