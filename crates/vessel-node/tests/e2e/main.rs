//! The node end to end: an in-process `vesseld` on a random loopback port
//! with a temporary data directory, driven only through `vessel-client`,
//! with testkit fakes standing in for the harnesses.
//!
//! Tests wait on records and replies, never on time; the timeout in
//! `support::within` only turns a hang into a failure, and the shorter
//! ones of `support::step` and the output readers say which wait it was.
//! A failing test's output holds the node's, relay's and client's debug
//! logs (`support::logs`).

#[cfg(test)]
mod across;
#[cfg(test)]
mod added;
#[cfg(test)]
mod agents;
#[cfg(test)]
mod attachments;
#[cfg(test)]
mod attachments_sent;
#[cfg(test)]
mod callback;
#[cfg(test)]
mod catalogue;
#[cfg(test)]
mod conversation;
#[cfg(test)]
mod crowd;
#[cfg(test)]
mod fleet;
#[cfg(test)]
mod forge;
#[cfg(test)]
mod forge_rules;
#[cfg(test)]
mod grants;
#[cfg(test)]
mod install;
#[cfg(test)]
mod isolation;
#[cfg(test)]
mod local;
#[cfg(test)]
mod lsp;
#[cfg(test)]
mod marks;
// Where protocol 11's pull requests and terminals meet (Unix: the
// terminals' shell is `/bin/sh`).
#[cfg(test)]
mod inbox;
#[cfg(test)]
#[cfg(unix)]
mod meeting;
#[cfg(test)]
mod moving;
#[cfg(test)]
mod notices;
#[cfg(test)]
mod options;
#[cfg(test)]
mod page_wire;
#[cfg(test)]
mod profiles;
#[cfg(test)]
mod prompts;
#[cfg(test)]
#[cfg(unix)]
mod remote;
#[cfg(test)]
#[cfg(unix)]
mod removal_race;
#[cfg(test)]
mod resume;
#[cfg(test)]
mod routing;
// Where track D's schedules meet pull requests, terminals and added
// agents (Unix: the terminals' shell is `/bin/sh`).
#[cfg(test)]
#[cfg(unix)]
mod scheduled;
#[cfg(test)]
mod schedules;
#[cfg(test)]
mod secrets_across;
#[cfg(test)]
mod secure;
#[cfg(test)]
mod status;
#[cfg(test)]
mod structured;
#[cfg(test)]
mod support;
#[cfg(test)]
mod terminal;
#[cfg(test)]
#[cfg(unix)]
mod terminals;
#[cfg(test)]
mod tidying;
#[cfg(test)]
mod transfers;
#[cfg(test)]
mod transfers_across;
#[cfg(test)]
mod waiting;
#[cfg(test)]
mod workspace;
