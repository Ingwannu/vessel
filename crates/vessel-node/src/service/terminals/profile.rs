//! `terminal.create` with a terminal profile (ADR 0117): the node finds the
//! profile in its own list and starts its program in the place's folder.
//!
//! [Decision Log]
//! - Purpose: Paseo's "Terminal profiles" rows open a terminal running an
//!   agent's CLI (`claude`, `codex`, …) where a shell would run.
//! - Reference: Paseo's client turns the profile into a command and its
//!   arguments, and the daemon runs them literally, with no shell, after
//!   checking the program is there (`server/src/terminal/terminal.ts`
//!   706-826); the tab is named after the profile.
//! - Choice: the client names the profile by id. The node looks it up in
//!   its list in force (kept, else shipped), adopts the prompt placeholder,
//!   puts the prompt in, finds the program, then starts it exactly as it
//!   starts a shell: the same folder, the same scrubbed environment and
//!   activity capability, the same pseudo-terminal. It is a shell's kind on
//!   the wire, so every client types into it.
//! - Why by id: one resolver for every client; a profile removed meanwhile
//!   is refused rather than run from a client's stale copy; the node runs
//!   only templates its admin saved. The person sees the same thing.
//! - Cost: a client cannot run an unsaved command line this way; it opens a
//!   shell and types it, as before.

use std::path::Path;

use vessel_proto::terminal_profiles::{ProfileLaunch, Prompt};
use vessel_proto::{ErrorCode, SessionTitle, WireError};

use super::super::NodeService;
use crate::terminals::{self, Shell, TerminalsError};

/// What a profile starts, in a shell's shape, and the name it gives the
/// terminal.
pub(super) struct Launch {
    /// The program found, with its resolved arguments.
    pub(super) program: Shell,
    /// The profile's name, when it is a valid title.
    pub(super) title: Option<SessionTitle>,
}

impl NodeService {
    /// Resolve `asked` for a terminal in `folder`, before anything is
    /// reserved or started.
    pub(super) async fn profile_launch(
        &self,
        asked: &ProfileLaunch,
        folder: &Path,
    ) -> Result<Launch, TerminalsError> {
        let profiles = self.settings.terminal_profiles().await;
        let Some(profile) = profiles.get(&asked.id) else {
            return Err(TerminalsError::Wire(WireError::new(
                ErrorCode::NotFound,
                format!("no terminal profile {}", asked.id.as_str()),
            )));
        };
        let invalid = |error: vessel_proto::terminal_profiles::ProfileError| {
            TerminalsError::Invalid(error.to_string())
        };
        let prompt = match &asked.prompt {
            Some(prompt) => prompt.clone(),
            None => Prompt::parse("").map_err(invalid)?,
        };
        let resolved = profile.resolve(&prompt).map_err(invalid)?;
        let command = resolved.command().as_str();
        let config = &self.terminals.runner.config;
        let lookup = terminals::Lookup {
            search: config.search_path.as_deref(),
            extensions: config.path_extensions.as_deref(),
            windows: cfg!(windows),
        };
        let Some(program) =
            terminals::find_program(command, folder, lookup, terminals::executable)
        else {
            // Paseo's `TerminalCommandNotFoundError` message.
            return Err(TerminalsError::Wire(WireError::new(
                ErrorCode::SpawnFailed,
                format!("{command}: command not found"),
            )));
        };
        let args: Vec<String> = resolved
            .args()
            .iter()
            .map(|arg| arg.as_str().to_owned())
            .collect();
        // cmd.exe would read a batch script's arguments again (`program`).
        if lookup.windows
            && terminals::batch_script(&program)
            && !args.iter().all(|arg| terminals::plain_for_batch(arg))
        {
            return Err(TerminalsError::Unsupported(format!(
                "{command} is a Windows batch script, which cmd.exe reads again; \
                 Vessel passes it only plain arguments (letters, digits and -_./:=+@\\). \
                 Name the program's .exe to give it a prompt"
            )));
        }
        Ok(Launch {
            program: Shell { program, args },
            title: profile.name().as_str().parse().ok(),
        })
    }
}
