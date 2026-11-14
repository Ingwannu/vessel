//! Meta's Muse Code (`muse`), a known agent of ADR 0058.
//!
//! Paseo drives it over its own protocol (`muse serve`); Vessel first uses
//! the ACP Meta's registry entry names (`muse acp` with
//! `MUSE_EXPERIMENTAL_ACP=on`), which Muse calls experimental, and adds the
//! direct driver behind ADR 0057's seam later. Meta's installer is shown,
//! not run (Paseo's own wording). No home is named. It gets the clean
//! environment and `META_API_KEY`, which Muse reads in place of a login.

use vessel_proto::Risk;

use super::{Descriptor, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;
use crate::modes::ModeTag;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "muse",
    profile: "muse",
    structured: Structured::Native {
        args: &["acp"],
        env: &[("MUSE_EXPERIMENTAL_ACP", "on")],
    },
    inherits: Inheritance::Clean {
        names: &["META_API_KEY"],
    },
    modes: &[
        ModeTag::asks("onRequest", Risk::Safe),
        ModeTag::asks("promptUnmatched", Risk::Moderate),
        ModeTag::asks("denyUnmatched", Risk::Safe),
        ModeTag::unattended("allowAll"),
    ],
    home: None,
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::ByHand("install Muse Code, so that `muse` is on the search path"),
    login_command: Some(&["login"]),
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
