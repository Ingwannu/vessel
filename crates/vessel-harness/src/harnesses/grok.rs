//! xAI's Grok Build CLI (`grok`), a known agent of ADR 0058.
//!
//! Driven as Paseo drives it, over ACP: `grok agent stdio`, the command of
//! xAI's own registry entry, with `--no-auto-update`, which xAI asks
//! automation to pass so a session never updates the binary under itself.
//! Its home is `~/.grok` (where its login is kept); no variable moving it
//! is documented, so a profile cannot choose another account by home. It
//! gets the clean environment and `XAI_API_KEY`, the key xAI documents.

use super::{Descriptor, Home, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "grok",
    profile: "grok",
    structured: Structured::Native {
        args: &["agent", "stdio", "--no-auto-update"],
        env: &[],
    },
    inherits: Inheritance::Clean {
        names: &["XAI_API_KEY"],
    },
    modes: &[],
    home: Some(Home {
        variable: None,
        default: &[".grok"],
    }),
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::Run("npm install -g @xai-official/grok"),
    login_command: Some(&["login"]),
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
