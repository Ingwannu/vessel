//! Pull requests on a forge (milestone 11, track B2, ADR 0029): the one
//! trait every forge implements ([`Forge`]), GitHub behind it
//! ([`GitHub`]), signing in as the machine already is ([`Credentials`]:
//! a token the binary found in the environment, or the machine's `gh`),
//! a git remote read into its repository ([`remote`]), and an in-memory
//! forge for tests (`fake`, feature `fake`).
//!
//! Owns: requests to a forge and reading its answers into the wire's
//! types. Must not: know sessions, principals or the store (the node's
//! `forge` module does), keep a token anywhere but memory, or log one.

mod gh;
mod github;
mod http;
mod provider;
pub mod remote;
mod token;

#[cfg(feature = "fake")]
pub mod fake;

pub use gh::{CloneProtocol, Gh};
pub use github::GitHub;
pub use provider::{
    Answer, Author, Comment, Discussion, FailedCheck, Forge, ForgeError, Found, Fresh, ItemId,
    Merged, Opened, Opening, Priority, Read, Review, ReviewVerdict, Thread,
};
pub use token::{Credentials, EnvTokens, Found as TokenFound, SIGN_IN_VARIABLES, Token};
