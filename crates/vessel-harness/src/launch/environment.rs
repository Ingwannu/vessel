//! What a launched harness's environment holds, in the order it is built:
//! where it starts from (the node's environment, or the clean one of ADR
//! 0058), the profile's changes, the model route's; and the check that an
//! MCP server's variables cannot change the harness itself.
//!
//! [Decision Log]
//! - Purpose: one place decides what a harness process inherits, so a
//!   launch, a login check and a version probe of the same agent agree.
//! - Split from `launch` (2026-11-14) along that seam: `launch` decides
//!   which program runs with which arguments, this what it inherits.

use std::ffi::OsString;

use vessel_proc::CommandSpec;
use vessel_proto::HarnessKind;

use crate::env::HarnessEnv;
use crate::harnesses::{Inheritance, descriptor};
use crate::lookup::Lookup;
use crate::mcp::{McpError, TerminalMcp};
use crate::profile::{Profile, same_variable};
use crate::routing::Routed;

/// `command` in the environment `harness` starts from: the node's own
/// (nothing to do), or the clean one with its vendor's variables when the
/// node has them (ADR 0058, `Inheritance`).
pub(crate) fn environment(
    command: CommandSpec,
    lookup: &Lookup,
    env: &HarnessEnv,
    harness: HarnessKind,
) -> CommandSpec {
    let names = match descriptor(harness).inherits {
        Inheritance::Node => return command,
        Inheritance::Clean { names } => names,
    };
    let node = |name: &str| std::env::var_os(name);
    let command = crate::added::clean(command, lookup, env, &node);
    names
        .iter()
        .fold(command, |command, name| match node(name) {
            Some(value) => command.env(name, value),
            None => command,
        })
}

/// `command` with `profile`'s environment changes: removals first, then
/// the variables it sets (a profile never does both to one variable).
pub(crate) fn with_profile(command: CommandSpec, profile: &Profile) -> CommandSpec {
    let command = profile
        .unset()
        .fold(command, |command, name| command.env_remove(name.as_str()));
    profile.env().fold(command, |command, (name, value)| {
        command.env(name.as_str(), value)
    })
}

/// `command` pointed at the model proxy: the route's removals, then its
/// variables.
pub(super) fn with_routing(command: CommandSpec, routed: &Routed) -> CommandSpec {
    let command = routed.unset.iter().fold(command, CommandSpec::env_remove);
    routed
        .env
        .iter()
        .fold(command, |command, (name, value)| command.env(name, value))
}

/// An MCP variable goes into the harness's own environment (Codex), so it
/// must not change the harness itself: no variable that steers it (its
/// home, search path, loader, proxies, credentials), and no variable the
/// harness would otherwise get with another value, from the profile or
/// from the environment it inherits (`inherited`, the node's own). A
/// variable the profile removes may not come back either.
pub(crate) fn check_environment(
    rendered: &TerminalMcp,
    profile: &Profile,
    home_variable: Option<&str>,
    inherited: &dyn Fn(&str) -> Option<OsString>,
) -> Result<(), McpError> {
    for (name, value) in &rendered.env {
        let same = |other: &str| same_variable(other, name.as_str());
        if protected(name.as_str()) || home_variable.is_some_and(same) {
            return Err(McpError::ProtectedVariable { name: name.clone() });
        }
        let profile_value = profile
            .env()
            .find(|(key, _)| same(key.as_str()))
            .map(|(_, set)| OsString::from(set));
        let removed = profile.unset().any(|key| same(key.as_str()));
        let current = profile_value.or_else(|| inherited(name.as_str()));
        let differs = current.is_some_and(|current| current != value.expose());
        if removed || differs {
            return Err(McpError::SharedVariable { name: name.clone() });
        }
    }
    Ok(())
}

/// Variables that steer a harness rather than configure an MCP server.
/// Compared ignoring case everywhere: `http_proxy` is as live as
/// `HTTP_PROXY`.
fn protected(name: &str) -> bool {
    let name = name.to_ascii_uppercase();
    matches!(
        name.as_str(),
        "PATH" | "PATHEXT" | "HOME" | "USERPROFILE" | "NODE_OPTIONS"
    ) || ["LD_", "DYLD_", "OPENAI_", "CODEX_"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
        || name.ends_with("_PROXY")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use vessel_proto::{EnvName, ResolvedValue};

    use super::*;
    use crate::profile::parse_profiles;

    fn rendered(pairs: &[(&str, &str)]) -> TerminalMcp {
        TerminalMcp {
            env: pairs
                .iter()
                .map(|(name, value)| {
                    (
                        name.parse::<EnvName>().unwrap(),
                        ResolvedValue::new((*value).to_owned()),
                    )
                })
                .collect(),
            ..TerminalMcp::default()
        }
    }

    fn profile(text: &str) -> Profile {
        let file = format!("[profiles.p]\nharness = \"codex\"\n{text}\n");
        parse_profiles(&file, Some(Path::new("/h")))
            .unwrap()
            .get(&"p".parse().unwrap())
            .cloned()
            .unwrap()
    }

    /// The node's environment, for these tests.
    fn node_env(name: &str) -> Option<OsString> {
        match name {
            "DOCS_TOKEN" => Some("inherited".into()),
            "SAME" => Some("same".into()),
            _ => None,
        }
    }

    fn check(pairs: &[(&str, &str)], profile: &Profile) -> Result<(), McpError> {
        check_environment(&rendered(pairs), profile, Some("CODEX_HOME"), &node_env)
    }

    fn shared(name: &str) -> McpError {
        McpError::SharedVariable {
            name: name.parse().unwrap(),
        }
    }

    fn protected_name(name: &str) -> McpError {
        McpError::ProtectedVariable {
            name: name.parse().unwrap(),
        }
    }

    /// Review finding 2: a server's variable must not override what Codex
    /// inherits.
    #[test]
    fn a_variable_the_harness_inherits_keeps_its_value() {
        let plain = Profile::default_for(HarnessKind::Codex);
        assert_eq!(
            check(&[("DOCS_TOKEN", "mine")], &plain),
            Err(shared("DOCS_TOKEN"))
        );
        assert_eq!(check(&[("SAME", "same")], &plain), Ok(()));
        assert_eq!(check(&[("FRESH", "x")], &plain), Ok(()));
        // The profile decides first: its value, or its removal.
        let sets = profile("env = { DOCS_TOKEN = \"mine\" }");
        assert_eq!(check(&[("DOCS_TOKEN", "mine")], &sets), Ok(()));
        assert_eq!(
            check(&[("DOCS_TOKEN", "other")], &sets),
            Err(shared("DOCS_TOKEN"))
        );
        let removes = profile("unset = [\"FRESH\"]");
        assert_eq!(check(&[("FRESH", "x")], &removes), Err(shared("FRESH")));
    }

    #[test]
    fn variables_that_steer_the_harness_are_refused() {
        let plain = Profile::default_for(HarnessKind::Codex);
        for name in [
            "PATH",
            "Path",
            "HOME",
            "USERPROFILE",
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "NODE_OPTIONS",
            "HTTPS_PROXY",
            "http_proxy",
            "OPENAI_API_KEY",
            "CODEX_HOME",
            "CODEX_SANDBOX",
        ] {
            assert_eq!(
                check(&[(name, "x")], &plain),
                Err(protected_name(name)),
                "{name}"
            );
        }
        assert_eq!(check(&[("PROXYLESS", "x")], &plain), Ok(()));
    }
}
