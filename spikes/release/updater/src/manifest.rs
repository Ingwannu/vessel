//! The feed's manifest: what the newest build of a channel is and where
//! its artifacts are, signed as one document.
//!
//! [Decision Log]
//! - Purpose: tell a copy of the app whether a newer build exists and
//!   exactly which bytes it is, so nothing it downloads can be other than
//!   what the release key signed.
//! - Constraints: the feed is a directory on any static HTTPS host (the
//!   owner has not chosen one), so the manifest must say everything; the
//!   key's public half is compiled in; a publish must never be seen half
//!   done.
//! - Alternatives: Tauri's `latest.json` signs each artifact and leaves
//!   the manifest (version, URLs) unsigned, so whoever controls the host
//!   can offer an older signed build as the latest; Zed signs nothing and
//!   trusts TLS and the systems' code signing, which Vessel does not have
//!   yet (no Windows certificate, no Apple account). A manifest and a
//!   detached `.minisig` beside it can be read between two writes of a
//!   publish and then fail to verify.
//! - Choice: one file per channel, `<channel>.json`, an envelope holding
//!   the manifest's exact text and its minisign signature. The manifest
//!   names the channel and the label, and gives every artifact's path,
//!   size and SHA-256. The app verifies the signature over the text,
//!   then parses it, then checks channel and label, and later checks
//!   each byte it downloads against the size and digest.
//! - Why: one signature covers the version, the channel and every
//!   artifact; one file is replaced in one rename, so a reader sees the
//!   old feed or the new one; the text is verified before it is parsed,
//!   so the JSON parser never sees unsigned input beyond the envelope.
//! - Cost: the manifest inside the envelope is a JSON string, less easy
//!   to read by eye (`vessel-app-feed check` prints it); an artifact
//!   cannot be checked by its own signature alone (`SHA256SUMS.minisig`
//!   beside the artifacts serves people checking a download by hand).
//!
//! Every bound below is a ceiling on what a hostile or broken feed can
//! make the app hold or show.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{
    channel::Channel,
    key::{KeyError, PublicKey},
    label::Label,
    platform::{Packaging, Target},
};

/// The largest envelope read from a feed. A manifest of 32 artifacts with
/// release notes fits in a few KiB.
pub const MAX_ENVELOPE_BYTES: u64 = 256 * 1024;

/// The most artifacts one manifest lists.
const MAX_ARTIFACTS: usize = 32;

/// The most companions one artifact carries (ADR 0094: the command line
/// and the node today).
const MAX_COMPANIONS: usize = 8;

/// The longest role name a later build may give a companion.
const MAX_ROLE: usize = 32;

/// The longest release notes shown to the owner.
const MAX_NOTES: usize = 8 * 1024;

/// The longest `published` text.
const MAX_PUBLISHED: usize = 64;

/// The largest artifact the app will download (the Windows executable is
/// tens of MiB; a macOS bundle of two architectures under 200).
pub const MAX_ARTIFACT_BYTES: u64 = 1024 * 1024 * 1024;

/// The manifest format this app reads. A feed written in a later format
/// is refused, so an old app tells its user to download a new one by hand
/// instead of misreading it.
pub const SCHEMA: u32 = 1;

/// What one `<channel>.json` holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    /// The manifest's exact text: the bytes the signature covers.
    pub manifest: String,
    /// A minisign signature of `manifest`, as minisign writes `.minisig`.
    pub signature: String,
}

/// The newest build of a channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    /// The format, [`SCHEMA`].
    pub schema: u32,
    /// The channel this manifest was signed for.
    pub channel: Channel,
    /// The build's label.
    pub label: Label,
    /// When it was published (RFC 3339, UTC), shown, never compared.
    pub published: Published,
    /// What changed, in the owner's words, shown before restarting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<Notes>,
    /// One per target and packaging.
    pub artifacts: Vec<Artifact>,
}

/// One downloadable build of the label.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// The system it runs on.
    pub target: Target,
    /// How it is installed, which is how it is replaced.
    pub packaging: Packaging,
    /// Where it is, relative to the feed's base.
    pub path: ArtifactPath,
    /// Its exact size.
    pub size: ByteCount,
    /// Its SHA-256.
    pub sha256: Sha256,
    /// The programs shipped beside the app and replaced with it, as one
    /// set (ADR 0094). Empty for the builds before it, and left out of the
    /// text then, so their manifests read as they always did.
    ///
    /// Compatibility, the reason this is a field and not a new format:
    /// every app published before ADR 0094 reads schema 1 with serde's
    /// default of ignoring fields it does not know (no manifest type here
    /// has ever denied unknown fields), so it still opens a manifest that
    /// carries companions and replaces its own executable as before. The
    /// build it installs then fills the companions in by itself
    /// ([`crate::service`]'s repair of the running label's set).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub companions: Vec<Companion>,
    /// The same file compressed, for apps that read this field
    /// ([`Compressed`]); published apps skip it and download `path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compressed: Option<Compressed>,
}

/// A file of the set compressed for the download.
///
/// Why (2026-11-15): an update of the owner's Windows copy over a slow
/// line broke off at 31.9 of 51.7 MB; gzip makes the executable about
/// 40% of its size. The file's own digest stays the artifact's (or
/// companion's) `sha256`; this one is the compressed bytes', checked
/// before they are unpacked, so nothing unsigned is ever decompressed, and
/// the unpacked file is checked against the file's digest again. An app
/// that does not know `encoding` downloads the plain file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compressed {
    /// How it is compressed.
    pub encoding: Encoding,
    /// Where it is, relative to the feed's base.
    pub path: ArtifactPath,
    /// Its exact size.
    pub size: ByteCount,
    /// The SHA-256 of the compressed bytes.
    pub sha256: Sha256,
}

/// A compression this build unpacks, or another (a later build's).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Encoding {
    /// gzip (RFC 1952), through `flate2`, already in the app's tree. zstd
    /// would be about a quarter smaller and adds a decoder crate.
    Gzip,
    /// Another encoding: this build downloads the plain file instead.
    Other(String),
}

impl TryFrom<String> for Encoding {
    type Error = RoleError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if text == "gzip" {
            return Ok(Self::Gzip);
        }
        // The same shape as a role's name: short, lowercase, plain.
        text.parse::<Role>().map(|_shape| Self::Other(text))
    }
}

impl From<Encoding> for String {
    fn from(encoding: Encoding) -> Self {
        match encoding {
            Encoding::Gzip => "gzip".to_owned(),
            Encoding::Other(name) => name,
        }
    }
}

/// A program shipped beside the app, downloaded and swapped together with
/// it (ADR 0094).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Companion {
    /// What it is, which names the file it is installed as.
    pub role: Role,
    /// Where it is, relative to the feed's base.
    pub path: ArtifactPath,
    /// Its exact size.
    pub size: ByteCount,
    /// Its SHA-256.
    pub sha256: Sha256,
    /// The same file compressed ([`Compressed`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compressed: Option<Compressed>,
}

/// The folder beside the app's executable that holds its companions.
///
/// Why a folder of their own (the coordinator's word on ADR 0094,
/// 2026-11-15): putting it on `PATH` gives `vessel` and `vesseld` without
/// the app itself, and on Windows the command line keeps its own name,
/// `vessel.exe`, which beside the app's `Vessel.exe` would be the same name
/// to a file system that ignores case.
pub const COMPANION_DIR: &str = "bin";

/// What a companion is. The file it is installed as comes from the role
/// and the system ([`Role::file_name`], in [`COMPANION_DIR`]), never from
/// the feed, so even a signed manifest cannot make the app write a file of
/// another name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Role {
    /// The command line, `vessel` (`vessel.exe` on Windows).
    Cli,
    /// The node, `vesseld`.
    Node,
    /// A role a later build ships and this one does not know: it parses,
    /// so the manifest still opens, and it is left out of the set.
    Other(String),
}

/// A role name that is not 1 to 32 lowercase letters, digits and `-`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a companion's role is 1 to {MAX_ROLE} lowercase letters, digits and -")]
pub struct RoleError;

impl Role {
    /// The file this role is installed as in [`COMPANION_DIR`] on
    /// `target`; `None` for a role this build does not install.
    #[must_use]
    pub const fn file_name(&self, target: Target) -> Option<&'static str> {
        let windows = matches!(target, Target::WindowsX86_64 | Target::WindowsAarch64);
        match (self, windows) {
            (Self::Cli, true) => Some("vessel.exe"),
            (Self::Cli, false) => Some("vessel"),
            (Self::Node, true) => Some("vesseld.exe"),
            (Self::Node, false) => Some("vesseld"),
            (Self::Other(_), _) => None,
        }
    }

    /// Every role this build installs.
    pub const KNOWN: [Self; 2] = [Self::Cli, Self::Node];
}

impl FromStr for Role {
    type Err = RoleError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "cli" => Ok(Self::Cli),
            "node" => Ok(Self::Node),
            _ => {
                let plain = text
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
                if !text.is_empty() && text.len() <= MAX_ROLE && plain {
                    Ok(Self::Other(text.to_owned()))
                } else {
                    Err(RoleError)
                }
            }
        }
    }
}

impl TryFrom<String> for Role {
    type Error = RoleError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Role> for String {
    fn from(role: Role) -> Self {
        role.to_string()
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Cli => "cli",
            Self::Node => "node",
            Self::Other(name) => name,
        })
    }
}

impl Manifest {
    /// The artifact for a copy of `target` installed as `packaging`.
    #[must_use]
    pub fn artifact(&self, target: Target, packaging: Packaging) -> Option<&Artifact> {
        self.artifacts
            .iter()
            .find(|artifact| artifact.target == target && artifact.packaging == packaging)
    }

    /// Check what a signature cannot: the format, the bounds, and that no
    /// target and packaging is listed twice.
    fn check(self) -> Result<Self, OpenError> {
        if self.schema != SCHEMA {
            return Err(OpenError::Schema { found: self.schema });
        }
        if self.artifacts.len() > MAX_ARTIFACTS {
            return Err(OpenError::Bounds);
        }
        for (index, artifact) in self.artifacts.iter().enumerate() {
            let twice = self
                .artifacts
                .iter()
                .skip(index.saturating_add(1))
                .any(|other| {
                    other.target == artifact.target && other.packaging == artifact.packaging
                });
            if twice {
                return Err(OpenError::Duplicate {
                    target: artifact.target,
                    packaging: artifact.packaging,
                });
            }
            if artifact.companions.len() > MAX_COMPANIONS {
                return Err(OpenError::Bounds);
            }
            for (at, companion) in artifact.companions.iter().enumerate() {
                let again = artifact
                    .companions
                    .iter()
                    .skip(at.saturating_add(1))
                    .any(|other| other.role == companion.role);
                if again {
                    return Err(OpenError::CompanionTwice {
                        target: artifact.target,
                        role: companion.role.clone(),
                    });
                }
            }
        }
        Ok(self)
    }

    /// Open an envelope: verify its signature over the manifest's text
    /// with `key`, then parse and check the manifest, then require it to
    /// be `channel`'s.
    ///
    /// # Errors
    /// [`OpenError`] for an envelope that is too large or not one, a
    /// signature that does not verify, a manifest that does not parse or
    /// breaks a bound, and a manifest of another channel.
    pub fn open(envelope: &[u8], key: &PublicKey, channel: &Channel) -> Result<Self, OpenError> {
        if u64::try_from(envelope.len()).map_or(true, |len| len > MAX_ENVELOPE_BYTES) {
            return Err(OpenError::Bounds);
        }
        let envelope: Envelope = serde_json::from_slice(envelope).map_err(OpenError::Envelope)?;
        key.verify(envelope.manifest.as_bytes(), &envelope.signature)
            .map_err(OpenError::Signature)?;
        let manifest: Self =
            serde_json::from_str(&envelope.manifest).map_err(OpenError::Manifest)?;
        let manifest = manifest.check()?;
        if &manifest.channel != channel {
            return Err(OpenError::Channel {
                found: manifest.channel,
            });
        }
        Ok(manifest)
    }
}

/// Why an envelope was not accepted.
#[derive(Debug, thiserror::Error)]
pub enum OpenError {
    /// Larger than [`MAX_ENVELOPE_BYTES`], or the manifest breaks a bound.
    #[error("the feed's manifest is larger than the app accepts")]
    Bounds,
    /// Not an envelope.
    #[error("the feed's file is not a signed manifest")]
    Envelope(#[source] serde_json::Error),
    /// The signature is not the release key's over this text.
    #[error("the feed's manifest is not signed by this app's release key")]
    Signature(#[source] KeyError),
    /// Signed, but not a manifest this app reads.
    #[error("the feed's signed manifest does not parse")]
    Manifest(#[source] serde_json::Error),
    /// Written in a format this app does not know.
    #[error(
        "the feed is in format {found}, this app reads {SCHEMA}: download the new build by hand"
    )]
    Schema {
        /// The manifest's format.
        found: u32,
    },
    /// Signed for another channel.
    #[error("the feed's manifest is for channel {found}")]
    Channel {
        /// The channel the manifest names.
        found: Channel,
    },
    /// Two artifacts for one target and packaging.
    #[error("the feed lists {target} {packaging} twice")]
    Duplicate {
        /// The target listed twice.
        target: Target,
        /// Its packaging.
        packaging: Packaging,
    },
    /// One artifact names a companion role twice: which file would win is
    /// not a question the app should answer.
    #[error("the feed lists the {role} companion of {target} twice")]
    CompanionTwice {
        /// The artifact's target.
        target: Target,
        /// The role listed twice.
        role: Role,
    },
}

/// A path under the feed's base: one to four `/`-separated names of
/// ASCII letters, digits, `.`, `_` and `-`, none `.` or `..`, at most 200
/// bytes. So it can only point inside the feed, and only to a file name
/// any static host serves as it is.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ArtifactPath(String);

/// Why a text is not an [`ArtifactPath`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an artifact's path is one to four plain names under the feed")]
pub struct ArtifactPathError;

impl ArtifactPath {
    /// The path, `/`-separated.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The last name: the file's own name.
    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.rsplit('/').next().unwrap_or(&self.0)
    }
}

impl FromStr for ArtifactPath {
    type Err = ArtifactPathError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let plain_name = |name: &str| {
            !name.is_empty()
                && name != "."
                && name != ".."
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        };
        let names = text.split('/').count();
        if text.len() <= 200 && (1..=4).contains(&names) && text.split('/').all(plain_name) {
            Ok(Self(text.to_owned()))
        } else {
            Err(ArtifactPathError)
        }
    }
}

impl TryFrom<String> for ArtifactPath {
    type Error = ArtifactPathError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<ArtifactPath> for String {
    fn from(path: ArtifactPath) -> Self {
        path.0
    }
}

impl fmt::Display for ArtifactPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A size in bytes, at most [`MAX_ARTIFACT_BYTES`].
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(try_from = "u64", into = "u64")]
pub struct ByteCount(u64);

/// A size larger than the app downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("an artifact is at most {MAX_ARTIFACT_BYTES} bytes")]
pub struct ByteCountError;

impl ByteCount {
    /// The size.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl TryFrom<u64> for ByteCount {
    type Error = ByteCountError;

    fn try_from(bytes: u64) -> Result<Self, Self::Error> {
        if bytes <= MAX_ARTIFACT_BYTES {
            Ok(Self(bytes))
        } else {
            Err(ByteCountError)
        }
    }
}

impl From<ByteCount> for u64 {
    fn from(count: ByteCount) -> Self {
        count.0
    }
}

/// A SHA-256 digest, written as 64 lowercase hexadecimal digits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256([u8; 32]);

/// A text that is not 64 lowercase hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a SHA-256 is 64 lowercase hexadecimal digits")]
pub struct Sha256Error;

impl Sha256 {
    /// A digest's bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Hash `bytes` at once (the feed tool and tests; the download hashes
    /// as it streams).
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        use sha2::Digest as _;
        Self(sha2::Sha256::digest(bytes).into())
    }
}

impl FromStr for Sha256 {
    type Err = Sha256Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digit = |byte: u8| match byte {
            b'0'..=b'9' => byte.checked_sub(b'0'),
            b'a'..=b'f' => byte
                .checked_sub(b'a')
                .and_then(|value| value.checked_add(10)),
            _ => None,
        };
        let text = text.as_bytes();
        if text.len() != 64 {
            return Err(Sha256Error);
        }
        let mut bytes = [0_u8; 32];
        let (pairs, _rest) = text.as_chunks::<2>();
        for (byte, [high, low]) in bytes.iter_mut().zip(pairs) {
            let high = digit(*high).ok_or(Sha256Error)?;
            let low = digit(*low).ok_or(Sha256Error)?;
            *byte = high
                .checked_mul(16)
                .and_then(|high| high.checked_add(low))
                .ok_or(Sha256Error)?;
        }
        Ok(Self(bytes))
    }
}

impl TryFrom<String> for Sha256 {
    type Error = Sha256Error;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Sha256> for String {
    fn from(digest: Sha256) -> Self {
        digest.to_string()
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256({self})")
    }
}

/// When a build was published: text of at most 64 bytes, shown as it is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Published(String);

/// Release notes: text of at most 8 KiB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Notes(String);

/// A text longer than its field allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the text is longer than the field allows")]
pub struct TooLong;

macro_rules! bounded_text {
    ($name:ident, $max:expr) => {
        impl $name {
            /// The text.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $name {
            type Error = TooLong;

            fn try_from(text: String) -> Result<Self, Self::Error> {
                if text.len() <= $max {
                    Ok(Self(text))
                } else {
                    Err(TooLong)
                }
            }
        }

        impl From<$name> for String {
            fn from(text: $name) -> Self {
                text.0
            }
        }
    };
}

bounded_text!(Published, MAX_PUBLISHED);
bounded_text!(Notes, MAX_NOTES);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::key::tests::TestKey;

    fn manifest(channel: &str, label: &str) -> Manifest {
        Manifest {
            schema: SCHEMA,
            channel: channel.parse().unwrap(),
            label: label.parse().unwrap(),
            published: Published::try_from("2026-11-13T00:00:00Z".to_owned()).unwrap(),
            notes: Some(Notes::try_from("첫 빌드".to_owned()).unwrap()),
            artifacts: vec![Artifact {
                target: Target::WindowsX86_64,
                packaging: Packaging::Executable,
                path: format!("{label}/vessel-{label}-windows-x64.exe")
                    .parse()
                    .unwrap(),
                size: ByteCount::try_from(3).unwrap(),
                sha256: Sha256::of(b"exe"),
                companions: Vec::new(),
                compressed: None,
            }],
        }
    }

    fn companion(role: Role, bytes: &[u8]) -> Companion {
        Companion {
            path: format!("0.0.0.0.2/vessel-0.0.0.0.2-windows-x64-{role}")
                .parse()
                .unwrap(),
            role,
            size: ByteCount::try_from(u64::try_from(bytes.len()).unwrap()).unwrap(),
            sha256: Sha256::of(bytes),
            compressed: None,
        }
    }

    /// The manifest types exactly as every app up to 0.0.0.0.18 has them
    /// (no `companions`, serde's default of ignoring unknown fields): what
    /// a published app parses a new manifest with.
    mod published {
        use serde::Deserialize;

        use crate::{
            channel::Channel,
            label::Label,
            manifest::{ArtifactPath, ByteCount, Notes, Published, Sha256},
            platform::{Packaging, Target},
        };

        #[derive(Debug, Deserialize)]
        pub(super) struct Manifest {
            pub(super) schema: u32,
            pub(super) channel: Channel,
            pub(super) label: Label,
            pub(super) published: Published,
            pub(super) notes: Option<Notes>,
            pub(super) artifacts: Vec<Artifact>,
        }

        #[derive(Debug, Deserialize)]
        pub(super) struct Artifact {
            pub(super) target: Target,
            pub(super) packaging: Packaging,
            pub(super) path: ArtifactPath,
            pub(super) size: ByteCount,
            pub(super) sha256: Sha256,
        }
    }

    #[test]
    fn an_app_published_before_companions_still_reads_a_manifest_with_them() {
        let mut new = manifest("preview", "0.0.0.0.2");
        new.artifacts[0].companions =
            vec![companion(Role::Cli, b"cli"), companion(Role::Node, b"node")];
        let text = serde_json::to_string(&new).unwrap();
        assert!(text.contains("\"companions\""));

        let old: published::Manifest = serde_json::from_str(&text).unwrap();
        assert_eq!(old.schema, SCHEMA);
        assert_eq!(old.channel, new.channel);
        assert_eq!(old.label, new.label);
        assert_eq!(old.published, new.published);
        assert_eq!(old.notes, new.notes);
        let [artifact] = old.artifacts.as_slice() else {
            panic!("{:?}", old.artifacts);
        };
        assert_eq!(artifact.target, new.artifacts[0].target);
        assert_eq!(artifact.packaging, new.artifacts[0].packaging);
        assert_eq!(artifact.path, new.artifacts[0].path);
        assert_eq!(artifact.size, new.artifacts[0].size);
        assert_eq!(artifact.sha256, new.artifacts[0].sha256);

        // And a manifest without companions is the text it always was.
        let plain = serde_json::to_string(&manifest("preview", "0.0.0.0.2")).unwrap();
        assert!(!plain.contains("companions"), "{plain}");
    }

    #[test]
    fn companions_open_with_their_roles_and_unknown_roles_stay_out_of_the_set() {
        let key = TestKey::new();
        let mut sent = manifest("preview", "0.0.0.0.2");
        sent.artifacts[0].companions = vec![
            companion(Role::Cli, b"cli"),
            companion(Role::Node, b"node"),
            companion("helper".parse().unwrap(), b"later"),
        ];
        let opened =
            Manifest::open(&key.envelope(&sent), &key.public(), &Channel::preview()).unwrap();
        assert_eq!(opened, sent);
        let names: Vec<_> = opened.artifacts[0]
            .companions
            .iter()
            .map(|companion| companion.role.file_name(Target::WindowsX86_64))
            .collect();
        assert_eq!(names, [Some("vessel.exe"), Some("vesseld.exe"), None]);
        assert_eq!(Role::Cli.file_name(Target::LinuxX86_64), Some("vessel"));
        assert_eq!(Role::Node.file_name(Target::LinuxX86_64), Some("vesseld"));

        for bad in ["", "Cli", "a/b", "..", "x".repeat(33).as_str()] {
            assert!(bad.parse::<Role>().is_err(), "{bad:?} accepted");
        }
    }

    #[test]
    fn a_role_twice_or_too_many_companions_are_refused() {
        let key = TestKey::new();
        let mut twice = manifest("preview", "0.0.0.0.2");
        twice.artifacts[0].companions =
            vec![companion(Role::Node, b"a"), companion(Role::Node, b"b")];
        let opened = Manifest::open(&key.envelope(&twice), &key.public(), &Channel::preview());
        assert!(
            matches!(
                opened,
                Err(OpenError::CompanionTwice {
                    role: Role::Node,
                    ..
                })
            ),
            "{opened:?}"
        );

        let mut many = manifest("preview", "0.0.0.0.2");
        many.artifacts[0].companions = (0..=MAX_COMPANIONS)
            .map(|index| companion(format!("r{index}").parse().unwrap(), b"x"))
            .collect();
        let opened = Manifest::open(&key.envelope(&many), &key.public(), &Channel::preview());
        assert!(matches!(opened, Err(OpenError::Bounds)), "{opened:?}");
    }

    #[test]
    fn a_signed_manifest_opens_and_finds_its_artifact() {
        let key = TestKey::new();
        let sent = manifest("preview", "0.0.0.0.2");
        let envelope = key.envelope(&sent);
        let opened = Manifest::open(&envelope, &key.public(), &Channel::preview()).unwrap();
        assert_eq!(opened, sent);
        let artifact = opened
            .artifact(Target::WindowsX86_64, Packaging::Executable)
            .unwrap();
        assert_eq!(
            artifact.path.file_name(),
            "vessel-0.0.0.0.2-windows-x64.exe"
        );
        assert!(
            opened
                .artifact(Target::LinuxX86_64, Packaging::Executable)
                .is_none()
        );
    }

    #[test]
    fn a_manifest_changed_after_signing_is_refused() {
        let key = TestKey::new();
        let envelope = key.envelope(&manifest("preview", "0.0.0.0.2"));
        let mut envelope: Envelope = serde_json::from_slice(&envelope).unwrap();
        envelope.manifest = envelope.manifest.replace("0.0.0.0.2", "0.0.0.0.9");
        let tampered = serde_json::to_vec(&envelope).unwrap();
        let opened = Manifest::open(&tampered, &key.public(), &Channel::preview());
        assert!(matches!(opened, Err(OpenError::Signature(_))), "{opened:?}");
    }

    #[test]
    fn another_keys_signature_is_refused() {
        let signer = TestKey::new();
        let other = TestKey::new();
        let envelope = signer.envelope(&manifest("preview", "0.0.0.0.2"));
        let opened = Manifest::open(&envelope, &other.public(), &Channel::preview());
        assert!(matches!(opened, Err(OpenError::Signature(_))), "{opened:?}");
    }

    #[test]
    fn another_channels_manifest_is_refused() {
        let key = TestKey::new();
        let envelope = key.envelope(&manifest("stable", "0.0.0.0.2"));
        let opened = Manifest::open(&envelope, &key.public(), &Channel::preview());
        assert!(
            matches!(opened, Err(OpenError::Channel { .. })),
            "{opened:?}"
        );
    }

    #[test]
    fn a_later_format_and_a_duplicate_are_refused() {
        let key = TestKey::new();
        let mut later = manifest("preview", "0.0.0.0.2");
        later.schema = 2;
        let opened = Manifest::open(&key.envelope(&later), &key.public(), &Channel::preview());
        assert!(
            matches!(opened, Err(OpenError::Schema { found: 2 })),
            "{opened:?}"
        );

        let mut twice = manifest("preview", "0.0.0.0.2");
        twice.artifacts.push(twice.artifacts[0].clone());
        let opened = Manifest::open(&key.envelope(&twice), &key.public(), &Channel::preview());
        assert!(
            matches!(opened, Err(OpenError::Duplicate { .. })),
            "{opened:?}"
        );
    }

    #[test]
    fn an_oversized_envelope_is_refused_before_parsing() {
        let key = TestKey::new();
        let huge = vec![b' '; usize::try_from(MAX_ENVELOPE_BYTES).unwrap() + 1];
        let opened = Manifest::open(&huge, &key.public(), &Channel::preview());
        assert!(matches!(opened, Err(OpenError::Bounds)), "{opened:?}");
    }

    #[test]
    fn artifact_paths_stay_inside_the_feed() {
        for good in [
            "a",
            "0.0.0.0.1/vessel-0.0.0.0.1-windows-x64.exe",
            "a/b/c/d",
            "v_1.tar.gz",
        ] {
            assert!(good.parse::<ArtifactPath>().is_ok(), "{good:?} refused");
        }
        for bad in [
            "",
            "/a",
            "a/",
            "a//b",
            "../a",
            "a/../b",
            "./a",
            "a/b/c/d/e",
            "a\\b",
            "C:a",
            "a b",
            "https://x/a",
            "%2e%2e/a",
            "a?b",
            "é",
        ] {
            assert!(bad.parse::<ArtifactPath>().is_err(), "{bad:?} accepted");
        }
        assert!("a".repeat(201).parse::<ArtifactPath>().is_err());
    }

    #[test]
    fn digests_round_trip_and_reject_what_is_not_one() {
        let digest = Sha256::of(b"vessel");
        assert_eq!(digest.to_string().parse::<Sha256>(), Ok(digest));
        assert!("ab".parse::<Sha256>().is_err());
        assert!(digest.to_string().to_uppercase().parse::<Sha256>().is_err());
        assert!("g".repeat(64).parse::<Sha256>().is_err());
        assert_eq!(
            Sha256::of(b"").to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sizes_and_texts_are_bounded() {
        assert!(ByteCount::try_from(MAX_ARTIFACT_BYTES).is_ok());
        assert!(ByteCount::try_from(MAX_ARTIFACT_BYTES + 1).is_err());
        assert!(Notes::try_from("가".repeat(MAX_NOTES)).is_err());
        assert!(Published::try_from("x".repeat(MAX_PUBLISHED + 1)).is_err());
    }
}
