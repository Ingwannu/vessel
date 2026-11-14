//! Oh My Pi (`omp`), Pi's fork, a known agent of ADR 0058.
//!
//! Like Pi it has no ACP; Paseo drives it over `omp --mode rpc-ui`, and so
//! will Vessel's direct driver, which until built leaves a structured
//! session refused. Its home is `~/.omp`; no variable moving it is
//! documented. Installed with Bun, as its project documents. It gets the
//! clean environment; the provider keys it reads are listed with its
//! driver.

use vessel_proto::Risk;

use super::{Descriptor, Home, Inheritance, Installer, Structured};
use crate::layout::HarnessLayout;
use crate::modes::ModeTag;

pub(super) const DESCRIPTOR: Descriptor = Descriptor {
    binary: "omp",
    profile: "omp",
    structured: Structured::None {
        needs: "Oh My Pi's own protocol (`omp --mode rpc-ui`)",
    },
    inherits: Inheritance::Clean { names: &[] },
    modes: &[
        ModeTag::unattended("full"),
        ModeTag::asks("write", Risk::Moderate),
        ModeTag::asks("ask", Risk::Safe),
    ],
    home: Some(Home {
        variable: None,
        default: &[".omp"],
    }),
    layout: HarnessLayout {
        instructions: None,
        skills: None,
    },
    login: None,
    install: Installer::Run("bun install -g @oh-my-pi/pi-coding-agent"),
    login_command: None,
    account_files: &[],
    sessions: None,
    memory: None,
    rebase: None,
    terminal_mcp: None,
    terminal_args: true,
    model_proxy: None,
};
