//! Cursor's agent CLI (`cursor-agent`), a known agent of ADR 0058.
//!
//! Driven as Paseo drives it, over ACP (`cursor-agent acp`). Cursor's own
//! installer is a script piped to a shell, which Vessel shows and never
//! runs. Where it keeps its login differs by system and is not documented,
//! so no home is named. It gets the clean environment and
//! `CURSOR_API_KEY`, the key Cursor documents for scripts.

use super::{Descriptor, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "cursor-agent",
    profile: "cursor",
    structured: Structured::Native {
        args: &["acp"],
        env: &[],
    },
    inherits: Inheritance::Clean {
        names: &["CURSOR_API_KEY"],
    },
    modes: &[],
    home: None,
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::ByHand("curl https://cursor.com/install -fsS | bash"),
    login_command: Some(&["login"]),
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
