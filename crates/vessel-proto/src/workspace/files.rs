//! Files (protocol 10): list, read, write, create, rename, delete, find by
//! name and search inside, every path inside its root, every answer
//! bounded.

use serde::{Deserialize, Serialize};

use super::git::WorkspaceRoot;
use super::path::RelPath;
use super::text::{ContentHash, FindQuery, GrepQuery, SearchKey};

/// What an entry of a folder is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// A file.
    File,
    /// A folder.
    Dir,
    /// A symbolic link (not followed).
    Symlink,
    /// Anything else (a socket, a device).
    Other,
    /// A kind a newer node knows.
    #[serde(other)]
    Unknown,
}

/// One entry of a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DirEntry {
    /// Its name.
    pub name: String,
    /// What it is.
    pub kind: EntryKind,
    /// Its size in bytes, for a file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    /// Whether git ignores it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ignored: bool,
}

/// A folder's entries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DirListing {
    /// Folders first, then by name; at most 5,000.
    pub entries: Vec<DirEntry>,
    /// Whether there were more.
    pub more: bool,
}

/// What a write expects to find, so it never overwrites a change it did
/// not see.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileVersion {
    /// Its size in bytes.
    pub size: u64,
    /// When it was last changed, in nanoseconds since the Unix epoch.
    pub modified: u64,
    /// SHA-256 of its content, for a file of at most 8 MiB: a change
    /// within one tick of a coarse clock keeps the size and the time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<ContentHash>,
}

/// What a page of a file holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileContent {
    /// UTF-8 text.
    Text {
        /// The page, cut back to a character boundary.
        text: String,
        /// Where the rest starts, when there is more.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        next: Option<u64>,
    },
    /// Not text (a NUL byte in its first 8 KiB, or not UTF-8): not sent.
    Binary,
    /// A kind a newer node knows.
    #[serde(other)]
    Unknown,
}

/// A page of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FileRead {
    /// The version read, for a later write.
    pub version: FileVersion,
    /// The page.
    pub content: FileContent,
}

/// What a write expects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WriteExpect {
    /// Nothing there: create only.
    Absent,
    /// This version: replace only it.
    Version {
        /// The version read.
        version: FileVersion,
    },
    /// Whatever is there, or nothing.
    Any,
}

/// A file whose name matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FoundFile {
    /// Its path.
    pub path: RelPath,
    /// The characters of `path` (counted in Unicode scalar values, from 0)
    /// that matched the query, for highlighting.
    pub positions: Vec<u32>,
}

/// What `file.find` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FoundFiles {
    /// Best first.
    pub matches: Vec<FoundFile>,
    /// How many files were searched.
    pub indexed: u32,
    /// Whether every file of the root was searched (the index has a
    /// bound).
    pub complete: bool,
    /// Whether a newer search from the same connection replaced this one
    /// before it finished (then `matches` is empty).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub superseded: bool,
}

/// How a search in files reads its query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GrepMode {
    /// As the text it is.
    #[default]
    Literal,
    /// As an extended regular expression.
    Regex,
}

/// Whether case matters in a search in files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "snake_case")]
pub enum GrepCase {
    /// It does.
    Sensitive,
    /// It does not.
    Insensitive,
    /// It does when the query has a capital letter.
    #[default]
    Smart,
}

/// One line that matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GrepMatch {
    /// The file.
    pub path: RelPath,
    /// The line, from 1.
    pub line: u32,
    /// Where the match starts on it, from 1, in Unicode scalar values.
    pub column: u32,
    /// The line, at most 512 characters.
    pub text: String,
    /// Whether it was longer and cut.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
}

/// What `file.grep` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct GrepResult {
    /// By file, then line.
    pub matches: Vec<GrepMatch>,
    /// Whether there were more.
    pub more: bool,
}

/// `file.list`: one folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ListDir {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which folder; the root when empty.
    #[serde(default)]
    pub dir: RelPath,
    /// List what git ignores too (marked).
    #[serde(default)]
    pub ignored: bool,
}

/// `file.read`: a page of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ReadFile {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which file.
    pub path: RelPath,
    /// From this byte.
    #[serde(default)]
    pub offset: u64,
    /// At most this many bytes (at most 1 MiB, 256 KiB when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// `file.write`: a whole text file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct WriteFile {
    /// Where.
    pub root: WorkspaceRoot,
    /// Which file.
    pub path: RelPath,
    /// Its new content, at most 1 MiB.
    pub text: String,
    /// What must be there now.
    pub expect: WriteExpect,
}

/// `file.create_dir`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CreateDir {
    /// Where.
    pub root: WorkspaceRoot,
    /// The new folder (its parent must exist).
    pub path: RelPath,
}

/// `file.rename`: move a file or folder within its root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RenamePath {
    /// Where.
    pub root: WorkspaceRoot,
    /// What.
    pub from: RelPath,
    /// To where (nothing may be there).
    pub to: RelPath,
}

/// `file.delete`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DeletePath {
    /// Where.
    pub root: WorkspaceRoot,
    /// What.
    pub path: RelPath,
    /// A folder with everything in it; only an empty one otherwise.
    #[serde(default)]
    pub recursive: bool,
}

/// `file.find`: files by name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FindFiles {
    /// Where.
    pub root: WorkspaceRoot,
    /// What to look for.
    pub query: FindQuery,
    /// The search this one replaces: a newer `file.find` with the same key
    /// from the same caller stops an older one still running.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<SearchKey>,
    /// At most this many (at most 200, 50 when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

/// `file.grep`: search inside files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SearchFiles {
    /// Where.
    pub root: WorkspaceRoot,
    /// What to look for.
    pub query: GrepQuery,
    /// How to read it.
    #[serde(default)]
    pub mode: GrepMode,
    /// Whether case matters.
    #[serde(default)]
    pub case: GrepCase,
    /// Only under this folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<RelPath>,
    /// At most this many matches (at most 500, 100 when absent).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}
