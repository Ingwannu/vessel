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
mod comparison;
#[cfg(test)]
mod conversation;
#[cfg(test)]
mod crowd;
#[cfg(test)]
mod device_invite;
#[cfg(test)]
mod device_self;
#[cfg(test)]
mod discovery_ingress;
#[cfg(test)]
mod export_metadata;
#[cfg(test)]
mod fleet;
#[cfg(test)]
mod fleet_control;
#[cfg(test)]
mod fleet_control_cancel;
#[cfg(test)]
mod fleet_published;
#[cfg(test)]
mod forge;
#[cfg(test)]
mod forge_rules;
// A half-open TCP path, for the keepalive tests.
#[cfg(test)]
mod freezer;
#[cfg(test)]
mod grants;
#[cfg(test)]
mod install;
#[cfg(test)]
mod isolation;
#[cfg(test)]
mod label_ingress;
#[cfg(test)]
mod local;
#[cfg(test)]
mod lsp;
#[cfg(test)]
mod marks;
// Where protocol 11's pull requests and terminals meet (Unix: the
// terminals' shell is `/bin/sh`).
#[cfg(test)]
mod forking;
#[cfg(test)]
mod history;
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
mod parking;
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
mod settings;
// Where track D's schedules meet pull requests, terminals and added
// agents (Unix: the terminals' shell is `/bin/sh`).
#[cfg(test)]
mod schedule_preview;
#[cfg(test)]
#[cfg(unix)]
mod scheduled;
#[cfg(test)]
mod schedules;
#[cfg(test)]
mod search;
#[cfg(test)]
mod secrets_across;
#[cfg(test)]
mod secure;
#[cfg(test)]
mod side_questions;
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

#[cfg(test)]
mod limits;
#[cfg(test)]
mod usage;

#[cfg(test)]
mod usage_replay;

#[cfg(test)]
mod timeline_forward;
