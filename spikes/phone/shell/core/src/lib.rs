//! The phone app's core (milestone 12, track M): Vessel's bridge run in
//! the app's own process for the app's web view, and the app's own book
//! of nodes.
//!
//! The app (`vessel-phone`, a Tauri 2 shell) gives this crate two private
//! directories and gets back the address its web view opens. The page is
//! the one `vessel ui` serves (ADR 0017), embedded in `vessel-web`; the
//! bridge is `vessel-web`'s, unchanged: loopback only, a fresh origin and
//! a one-use bootstrap per launch. What differs from `vessel ui` is only
//! where the nodes come from ([`PhoneSource`]): a phone has no node of its
//! own, no SSH and no CLI book, so it keeps a book of the nodes it paired
//! with, in the app's private storage.
//!
//! [Decision Log]
//! - Purpose: the owner asked for an installed app on the phone, "as far
//!   as it can be made to go"; ADR 0035 names a Tauri 2 shell showing the
//!   page as the phone's fallback where the native UI cannot reach.
//! - Constraint: no Tauri type reaches the code that decides anything
//!   (the milestone's rule); a phone cannot run `ssh`, has no `vesseld`
//!   and no home directory in the CLI's sense.
//! - Alternatives: the page talking to Rust over Tauri's IPC instead of a
//!   loopback websocket (ADR 0020's proposal: a second transport in the
//!   page, which other workers hold today, and a command surface in the
//!   web view); a hosted page (ADR 0017 refused it).
//! - Choice: the bridge as `vessel ui` runs it, in-process, bound to
//!   loopback on a port the system picks; the web view gets the bootstrap
//!   URL from Rust and no Tauri command at all.
//! - Cost: a loopback port is reachable by other apps on the phone; the
//!   bridge's own defences (a random host name per launch, the one-use
//!   bootstrap, mutual proofs on the socket) are what keep them out, as
//!   on a desktop where other users' programs share loopback.
//!
//! Modules:
//! - `bridge`: [`Bridge`], the server started and stopped;
//! - `source`: [`PhoneSource`], the book, pairing and how a node is reached;
//! - `connect`: the connector for each kind of book entry a phone can use;
//! - `roots`: the certificates `wss://` relays are checked against.

mod bridge;
mod connect;
mod roots;
mod source;

pub use bridge::{Bridge, LaunchUrl, StartError, StopError};
pub use connect::{NodeConnectError, NodeConnector};
pub use roots::{Roots, RootsError};
pub use source::{AppDirs, PhoneSource, SourceError};
