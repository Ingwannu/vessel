//! Whether a machine in the book is this computer, the one Paseo's Browse
//! is offered on (TRACKER 816, docs/plans/wp-add-project-flow.md ruling 5).
//!
//! [Decision Log]
//! - Purpose: Add project's Browse opens this computer's own folder chooser,
//!   so it may only be offered on a machine that sees this computer's
//!   files. Paseo knows it: its desktop app runs its own daemon.
//! - Constraint: Vessel's app runs no node of its own; it reaches every
//!   machine through the book. It knows what each node reported
//!   (`NodeInfo`: hostname, OS) and its own hostname and OS, read by the
//!   same `hostname` crate the node uses.
//! - Alternatives: a loopback route (ruling 5's first draft): a loopback
//!   port can be an SSH tunnel to another machine or a container's
//!   published port, both with other files, and their nodes report another
//!   hostname; a field on `board::Machine` (built in about 26 files of
//!   other tracks); reading a local node's data folder (its place is the
//!   node's choice).
//! - Choice (coordinator, 2026-11-20): the same OS and the same hostname,
//!   ASCII case aside. Routes play no part.
//! - Cost: another machine with the same hostname and OS is taken for this
//!   one; a path picked here that it lacks gets its `project.add`'s
//!   "Directory not found". The same OS keeps out a WSL node on Windows.

/// A computer as it names itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Host<'a> {
    /// Its OS hostname (`NodeInfo::hostname`, or this computer's).
    pub hostname: &'a str,
    /// Its OS (`std::env::consts::OS`).
    pub os: &'a str,
}

/// Whether a node that reported `node` runs on `own`, this computer.
#[must_use]
pub fn is_this_computer(node: Host<'_>, own: Host<'_>) -> bool {
    let hostname = node.hostname.trim();
    node.os == own.os && !hostname.is_empty() && hostname.eq_ignore_ascii_case(own.hostname.trim())
}

#[cfg(test)]
mod tests;
