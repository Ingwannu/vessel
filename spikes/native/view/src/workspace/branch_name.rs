//! A branch name checked as it is typed, with git's own rules
//! (`check-ref-format --branch`, the wire's `BranchName`), so a field says
//! what is wrong before anything is sent. The node checks again; this only
//! saves a round trip and says it in words (the page's
//! `data/workspace/branch-name.ts`, the same rules in the same order).

/// The longest name the node takes, in bytes.
const MAX_BYTES: usize = 255;

/// Why a name is not a branch name git takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchNameProblem {
    /// Nothing typed.
    Empty,
    /// Over 255 bytes.
    TooLong,
    /// `@` or `HEAD`.
    Reserved(&'static str),
    /// Starts with `-`.
    LeadingDash,
    /// A space or a control character.
    SpaceOrControl,
    /// One of `~^:?*[\`.
    Character(char),
    /// `..` or `@{`.
    Sequence(&'static str),
    /// Starts or ends with `/`, or holds `//`.
    Slashes,
    /// Ends with `.`.
    TrailingDot,
    /// A part starts with `.`.
    DotPart,
    /// A part ends with `.lock`.
    LockPart,
}

/// Why `name` is not a branch name git takes (`None`: it is).
#[must_use]
pub fn branch_name_problem(name: &str) -> Option<BranchNameProblem> {
    if name.is_empty() {
        return Some(BranchNameProblem::Empty);
    }
    if name.len() > MAX_BYTES {
        return Some(BranchNameProblem::TooLong);
    }
    match name {
        "@" => return Some(BranchNameProblem::Reserved("@")),
        "HEAD" => return Some(BranchNameProblem::Reserved("HEAD")),
        _other => {}
    }
    if name.starts_with('-') {
        return Some(BranchNameProblem::LeadingDash);
    }
    for character in name.chars() {
        if character <= ' ' || character == '\u{7f}' {
            return Some(BranchNameProblem::SpaceOrControl);
        }
        if "~^:?*[\\".contains(character) {
            return Some(BranchNameProblem::Character(character));
        }
    }
    if name.contains("..") {
        return Some(BranchNameProblem::Sequence(".."));
    }
    if name.contains("@{") {
        return Some(BranchNameProblem::Sequence("@{"));
    }
    if name.starts_with('/') || name.ends_with('/') || name.contains("//") {
        return Some(BranchNameProblem::Slashes);
    }
    if name.ends_with('.') {
        return Some(BranchNameProblem::TrailingDot);
    }
    for part in name.split('/') {
        if part.starts_with('.') {
            return Some(BranchNameProblem::DotPart);
        }
        // Git's rule is case-sensitive: `topic.LOCK` is a name it takes.
        if part.as_bytes().ends_with(b".lock") {
            return Some(BranchNameProblem::LockPart);
        }
    }
    None
}
