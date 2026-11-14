//! Amp (`amp`), a known agent of ADR 0058.
//!
//! Amp has no ACP of its own; Paseo lists `amp-acp`, an adapter a third
//! party makes, and so does Vessel, saying so wherever it is offered. The
//! adapter is a release download, shown and never run. Amp's own CLI is
//! the npm package Amp documents. No home is named (not documented for the
//! adapter's use). It gets the clean environment, `AMP_API_KEY` (Amp's key
//! for scripts) and `AMP_CLI_PATH` (where the adapter finds `amp`).

use super::{Descriptor, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "amp",
    profile: "amp",
    structured: Structured::Adapter {
        binary: "amp-acp",
        install: Installer::ByHand(
            "install amp-acp from https://github.com/tao12345666333/amp-acp (a third party's adapter)",
        ),
        third_party: true,
    },
    inherits: Inheritance::Clean {
        names: &["AMP_API_KEY", "AMP_CLI_PATH"],
    },
    modes: &[],
    home: None,
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::Run("npm install -g @sourcegraph/amp"),
    login_command: None,
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
