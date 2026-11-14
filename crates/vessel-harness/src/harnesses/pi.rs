//! Pi (`pi`), a known agent of ADR 0058.
//!
//! Pi has no ACP of its own; Paseo drives it over its JSON lines mode
//! (`pi --mode rpc`), and so will Vessel's direct driver. Until that driver
//! is built a structured session of Pi is refused, naming it. Its home is
//! `~/.pi/agent`, moved by `PI_CODING_AGENT_DIR`, so a profile can be
//! another account. A person signs in inside Pi. It gets the clean
//! environment; the provider keys Pi reads are listed with its driver,
//! and until then a profile sets the one it needs.

use super::{Descriptor, Home, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "pi",
    profile: "pi",
    structured: Structured::None {
        needs: "Pi's own protocol (`pi --mode rpc`)",
    },
    inherits: Inheritance::Clean { names: &[] },
    modes: &[],
    home: Some(Home {
        variable: Some("PI_CODING_AGENT_DIR"),
        default: &[".pi", "agent"],
    }),
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::Run("npm install -g @earendil-works/pi-coding-agent"),
    login_command: None,
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
