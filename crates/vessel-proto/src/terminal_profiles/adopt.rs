//! A profile that runs a shipped agent's program without the prompt
//! placeholder takes it on read (ADR 0117; Paseo's `adoptPromptSentinel`,
//! `protocol/src/terminal-profiles.ts` 136-324).
//!
//! [Decision Log]
//! - Purpose: a person's own "Claude" profile (`claude --model x`) gets the
//!   prompt of a new workspace as the shipped one does, without them having
//!   to know the placeholder.
//! - Reference: keyed on the command's base name, never the id (Settings
//!   makes ids); the appended form is the shipped default's for that
//!   program (`--prompt=` for `opencode`, which reads a trailing positional as
//!   a folder). A profile that already takes the prompt, or has any
//!   positional (a subcommand, a prompt, `--`), is left alone; the values of
//!   the options a CLI is known to take are skipped first. Nothing is written
//!   back: Settings shows and saves the list as kept.
//! - Choice: the same rule here, applied by the node when it launches, so
//!   every client gets one behaviour. The option table is Paseo's, data
//!   only; a name not in it is treated conservatively, as there.
//! - Cost: a CLI's new option that takes a value blocks adoption until it is
//!   listed, as in Paseo.

use super::{Argument, PROMPT_SENTINEL, Profile, Profiles};

/// Options whose value is the next argv entry, per base name (Paseo's
/// `GLOBAL_OPTIONS_WITH_VALUES_BY_COMMAND`, 148-254).
const OPTIONS_WITH_VALUES: [(&str, &[&str]); 4] = [
    (
        "claude",
        &[
            "--add-dir",
            "--agent",
            "--agents",
            "--allowed-tools",
            "--allowedTools",
            "--append-system-prompt",
            "--betas",
            "--debug-file",
            "--disallowed-tools",
            "--disallowedTools",
            "--effort",
            "--fallback-model",
            "--file",
            "--input-format",
            "--json-schema",
            "--max-budget-usd",
            "--mcp-config",
            "--model",
            "--name",
            "--output-format",
            "--permission-mode",
            "--plugin-dir",
            "--plugin-url",
            "--remote-control-session-name-prefix",
            "--settings",
            "--system-prompt",
            "--system-prompt-file",
            "--tools",
            "-n",
        ],
    ),
    (
        "codex",
        &[
            "--add-dir",
            "--ask-for-approval",
            "--cd",
            "--config",
            "--disable",
            "--enable",
            "--image",
            "--local-provider",
            "--model",
            "--profile",
            "--remote",
            "--remote-auth-token-env",
            "--sandbox",
            "-C",
            "-a",
            "-c",
            "-i",
            "-m",
            "-p",
            "-s",
        ],
    ),
    (
        "opencode",
        &[
            "--agent",
            "--cors",
            "--hostname",
            "--log-level",
            "--mdns-domain",
            "--model",
            "--port",
            "--prompt",
            "--replay-limit",
            "--session",
            "-m",
            "-s",
        ],
    ),
    (
        "pi",
        &[
            "--api-key",
            "--append-system-prompt",
            "--exclude-tools",
            "--export",
            "--extension",
            "--fork",
            "--mode",
            "--model",
            "--models",
            "--name",
            "--prompt-template",
            "--provider",
            "--session",
            "--session-dir",
            "--session-id",
            "--skill",
            "--system-prompt",
            "--theme",
            "--thinking",
            "--tools",
            "-e",
            "-n",
            "-t",
            "-xt",
        ],
    ),
];

/// A program's base name: past the last `/` or `\`, lowercased, before its
/// first `.` unless the name starts with one (Paseo's `getCommandBaseName`,
/// 117-124), so `/usr/local/bin/codex` and `codex.cmd` are both `codex`.
#[must_use]
pub fn base_name(command: &str) -> String {
    let last = command.rsplit(['/', '\\']).next().unwrap_or(command);
    let lower = last.to_lowercase();
    match lower.split_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem.to_owned(),
        Some(_) | None => lower,
    }
}

/// Whether `args` hold a positional: anything not an option, or `--`, past
/// the values of the options `base` is known to take (Paseo's
/// `hasPositionalArg`, 279-301).
fn has_positional(base: &str, args: &[Argument]) -> bool {
    let with_values = OPTIONS_WITH_VALUES
        .iter()
        .find(|(name, _)| *name == base)
        .map_or(&[][..], |(_, options)| *options);
    let mut value_next = false;
    for arg in args.iter().map(Argument::as_str) {
        if value_next {
            value_next = false;
            continue;
        }
        if arg == "--" || !arg.starts_with('-') {
            return true;
        }
        value_next = !arg.contains('=') && with_values.contains(&arg);
    }
    false
}

impl Profile {
    /// This profile with the shipped prompt arguments appended when it runs
    /// a shipped agent's program without them; else itself.
    #[must_use]
    pub fn adopted(&self) -> Self {
        let base = base_name(self.command().as_str());
        let Ok(shipped) = Profiles::shipped() else {
            return self.clone();
        };
        let prompt_args: Vec<Argument> = shipped
            .entries()
            .iter()
            .find(|profile| base_name(profile.command().as_str()) == base)
            .map(|profile| {
                profile
                    .args()
                    .iter()
                    .filter(|arg| arg.as_str().contains(PROMPT_SENTINEL))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        if prompt_args.is_empty() || self.takes_prompt() || has_positional(&base, self.args()) {
            return self.clone();
        }
        let mut args = self.args().to_vec();
        args.extend(prompt_args);
        // Over a bound with the arguments added: launch it as kept.
        Self::new(
            self.id().clone(),
            self.name().clone(),
            self.command().clone(),
            args,
            self.icon().cloned(),
        )
        .unwrap_or_else(|_| self.clone())
    }
}

impl Profiles {
    /// Every profile adopted, in order (Paseo's `resolveTerminalProfiles`
    /// over a kept list, 316-324).
    #[must_use]
    pub fn adopted(&self) -> Self {
        let adopted = self.entries().iter().map(Profile::adopted).collect();
        // The same ids and count: valid whenever this list is.
        Self::new(adopted).unwrap_or_else(|_| self.clone())
    }
}
