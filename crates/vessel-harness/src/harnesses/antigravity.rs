//! Google Antigravity's CLI (`agy`), a known agent of ADR 0058.
//!
//! It has no ACP of its own (Google's ACP server is a separate download,
//! reachable as an added agent); Paseo drives `agy` over its stream-json
//! mode, and so will Vessel's direct driver, which until built leaves a
//! structured session refused. Its installer is Google's, shown and never
//! run (Paseo's wording); a person signs in by running `agy`. No home is
//! named. It gets the clean environment.

use super::{Descriptor, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "agy",
    profile: "antigravity",
    structured: Structured::None {
        needs: "Antigravity's stream (`agy --output-format stream-json`)",
    },
    inherits: Inheritance::Clean { names: &[] },
    home: None,
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::ByHand("install Antigravity, then run `agy` once and sign in"),
    login_command: None,
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
