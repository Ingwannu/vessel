//! What Settings › About lists: the software the app is built from and the
//! licence each is under, as the page's (`web/src/data/about.ts`). A list
//! kept by hand, checked against shared and member manifests by its test,
//! so a dependency added without a line here fails `cargo test` rather
//! than going unlisted (the page's test checks `package.json`).
//!
//! [Decision Log]
//! - Purpose: the page's licences, for the app's own parts.
//! - Choice: shared dependencies by their published names in manifest order,
//!   member-local runtime declarations alphabetically, then the bundled faces;
//!   each licence as
//!   its crate declares it (`cargo metadata`), the faces' from their OFL
//!   texts beside them.

mod words;

/// What a part does in the app (its words are in `locales/*/about.ftl`).
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
    /// Resolving the app's translated messages.
    Translations,
    /// Making unique identifiers for domain values.
    Identifiers,
    /// The system's language.
    Language,
    /// The system's desktop settings.
    Desktop,
    /// Asynchronous tasks and change streams.
    Tasks,
    /// Raster images and vector drawing.
    Images,
    /// Errors.
    Errors,
    /// The microphone and speaker devices.
    AudioDevices,
    /// Processing audio samples, including echo cancellation.
    AudioProcessing,
    /// The network and its tasks.
    Network,
    /// Logs.
    Logs,
    /// Find's folding of accents.
    Accents,
    /// Pairing links as QR codes.
    QrCodes,
    /// Reading a terminal program's output.
    Terminal,
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

/// Runtime dependencies in the shared manifest's order, then member-local
/// external declarations alphabetically, then the faces.
pub const BUNDLED: &[Bundled] = &[
    krate("fluent-bundle", "Apache-2.0 OR MIT", Role::Translations),
    krate("fluent-syntax", "Apache-2.0 OR MIT", Role::Translations),
    krate("fluent-pseudo", "Apache-2.0 OR MIT", Role::Translations),
    krate("intl-memoizer", "Apache-2.0 OR MIT", Role::Translations),
    krate("unic-langid", "MIT OR Apache-2.0", Role::Translations),
    krate("uuid", "Apache-2.0 OR MIT", Role::Identifiers),
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
    krate("cpal", "Apache-2.0", Role::AudioDevices),
    krate("sonora", "BSD-3-Clause", Role::AudioProcessing),
    krate("tokio", "MIT", Role::Network),
    krate("tracing", "MIT", Role::Logs),
    krate("tracing-subscriber", "MIT", Role::Logs),
    krate(
        "unicode-normalization",
        "(MIT OR Apache-2.0) AND Unicode-3.0",
        Role::Accents,
    ),
    krate("vte", "Apache-2.0 OR MIT", Role::Terminal),
    krate("unicode-width", "MIT OR Apache-2.0", Role::Terminal),
    krate("qrcode", "MIT OR Apache-2.0", Role::QrCodes),
    krate("ashpd", "MIT", Role::Desktop),
    krate("futures-lite", "Apache-2.0 OR MIT", Role::Tasks),
    krate("image", "MIT OR Apache-2.0", Role::Images),
    krate("resvg", "Apache-2.0 OR MIT", Role::Images),
    face("Geist", Role::Face),
    face("Geist Mono", Role::CodeFace),
    face("Pretendard", Role::Hangul),
];

/// Vessel's own licence (`LICENSE.md`, ADR 0014).
pub const VESSEL_LICENCE: &str = "PolyForm Strict 1.0.0";

#[cfg(test)]
#[path = "about/tests.rs"]
mod tests;
