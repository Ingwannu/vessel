//! Vessel's web UI (ADR 0017): the local server `vessel ui` runs.
//!
//! It serves the single-page app embedded at build time (`build.rs`) on
//! loopback, and is the browser's only way to the fleet: the browser
//! redeems a one-use code ([`LaunchToken`]) for a session with mutual proofs,
//! and never holds a node's credentials or speaks Noise.
//!
//! Modules:
//! - `auth`: one-use bootstrap and role-separated HMAC session proofs;
//! - `transport`: counted TCP admission, bounded headers and shutdown;
//! - `server`: the router, loopback-only serving, the HTTP API;
//! - `guard`: the `Host` and `Origin` checks and the security headers on
//!   every response;
//! - `files`: the embedded files, with the app's routes falling back to
//!   `index.html`, and their caching (`hashed`: which are content-hashed)
//!   and encodings (`encoding`: which compressed form a request gets);
//! - `socket`: the browser's websocket: the hello within a deadline, then
//!   the bridge;
//! - `bridge`: one tab's calls, streams and ops;
//! - `calls`: a browser's call to a node, typed, checked and forwarded;
//! - `streams`: one tab's followed sessions and how each ends;
//! - `ops`: the ops' answers (the book's nodes, pairing, QR codes, the
//!   book's edits);
//! - `registry`: one connection per node, shared by every tab, with its
//!   state pushed to them;
//! - `keep`: the page's preferences and drafts, one task per file, kept
//!   where the source says ([`PageFiles`], ADR 0028);
//! - `paint`: the preferences' appearance written into the index, so the
//!   first paint is already right;
//! - [`NodeSource`]: the seam through which the CLI gives the nodes;
//! - [`wire`]: the frames the browser and the server exchange.
//!
//! This crate must not know how a book entry becomes a connection (tokens,
//! SSH, credentials): that is the CLI's, given through [`NodeSource`].

mod auth;
mod bridge;
mod calls;
mod encoding;
mod files;
mod guard;
#[cfg(test)]
mod hashed;
mod keep;
mod ops;
mod paint;
mod registry;
mod server;
mod socket;
mod source;
mod streams;
mod submission;
mod token;
mod transport;
pub mod wire;

pub use server::{ServeError, UiConfig, serve};
pub use source::{NodeSource, PageFiles};
pub use token::{LaunchToken, MintError};
