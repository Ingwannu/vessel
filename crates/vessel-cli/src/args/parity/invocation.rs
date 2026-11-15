//! Compatibility decoding at argv's edge preserves legacy terminal invocation forms.
//! This chooses a clap grammar only; it never opens a node or inspects credentials.
use std::ffi::OsString;

/// Root Paseo run/send spellings share their agent handlers without a second schema.
pub(crate) fn normalize(mut arguments: Vec<OsString>) -> Vec<OsString> {
    let Some(index) = root_command(&arguments) else {
        return arguments;
    };
    let Some(command) = arguments.get(index).and_then(|text| text.to_str()) else {
        return arguments;
    };
    let tail = arguments.get(index.saturating_add(1)..).unwrap_or_default();
    let explicit_provider = has_flag(tail, "--provider");
    let prompt_flags = [
        "--prompt",
        "--prompt-file",
        "--behavior",
        "--no-wait",
        "--image",
    ]
    .iter()
    .any(|flag| has_flag(tail, flag));
    let output_flags = ["--json", "--format", "-o", "--quiet", "-q", "--no-headers"]
        .iter()
        .any(|flag| has_flag(&arguments, flag));
    let use_agent = match command {
        "import" => true,
        "run" => explicit_provider,
        // Both grammars are ID-first. Explicit parity/output flags select the
        // parity policy; unflagged legacy terminal sends retain their contract.
        "send" => prompt_flags || output_flags,
        _ => false,
    };
    if use_agent {
        arguments.insert(index, OsString::from("agent"));
    }
    arguments
}
fn has_flag(arguments: &[OsString], flag: &str) -> bool {
    arguments
        .iter()
        .take_while(|arg| arg.to_str() != Some("--"))
        .filter_map(|arg| arg.to_str())
        .any(|arg| {
            arg == flag
                || arg.strip_prefix(flag).is_some_and(|rest| {
                    rest.starts_with('=') || (flag.len() == 2 && !rest.is_empty())
                })
        })
}
fn root_command(arguments: &[OsString]) -> Option<usize> {
    let mut skip_value = false;
    for (index, arg) in arguments.iter().enumerate().skip(1) {
        if skip_value {
            skip_value = false;
            continue;
        }
        let text = arg.to_str()?;
        if text == "--" {
            return None;
        }
        if text.starts_with('-') {
            skip_value = matches!(
                text,
                "--node"
                    | "--token"
                    | "--data-dir"
                    | "--config-dir"
                    | "--host"
                    | "--home"
                    | "--format"
                    | "-o"
            );
            continue;
        }
        return Some(index);
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    fn argv(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }
    #[test]
    fn provider_prompts_route_after_global_values_but_harness_arguments_do_not() {
        assert_eq!(
            normalize(argv(&[
                "vessel",
                "--node",
                "run",
                "run",
                "explain this",
                "--provider",
                "claude"
            ])),
            argv(&[
                "vessel",
                "--node",
                "run",
                "agent",
                "run",
                "explain this",
                "--provider",
                "claude"
            ])
        );
        let legacy = argv(&["vessel", "run", "claude", "--", "--provider", "codex"]);
        assert_eq!(normalize(legacy.clone()), legacy);
    }
    #[test]
    fn id_first_send_uses_flags_to_select_policy_without_guessing_from_prompt_text() {
        let legacy = argv(&["vessel", "send", "ab12", "hello"]);
        assert_eq!(normalize(legacy.clone()), legacy);
        assert_eq!(
            normalize(argv(&["vessel", "send", "ab12", "cafe"])),
            argv(&["vessel", "send", "ab12", "cafe"])
        );
        assert_eq!(
            normalize(argv(&["vessel", "send", "ab12", "--prompt", "hello"])),
            argv(&["vessel", "agent", "send", "ab12", "--prompt", "hello"])
        );
        for flag in ["--json", "-ojson", "--quiet"] {
            assert_eq!(
                normalize(argv(&["vessel", flag, "send", "ab12", "cafe"])),
                argv(&["vessel", flag, "agent", "send", "ab12", "cafe"])
            );
        }
    }
}
