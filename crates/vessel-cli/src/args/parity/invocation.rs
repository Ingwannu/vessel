//! Compatibility decoding at argv's edge preserves legacy terminal invocation forms.
//! This chooses a clap grammar only; it never opens a node or inspects credentials.
use crate::selector::SessionSelector;
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
    let legacy_selector = tail
        .first()
        .and_then(|text| text.to_str())
        .is_some_and(|text| text.parse::<SessionSelector>().is_ok());
    let use_agent = match command {
        "import" => true,
        "run" => explicit_provider,
        "send" => prompt_flags || !legacy_selector,
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
                || arg
                    .strip_prefix(flag)
                    .is_some_and(|rest| rest.starts_with('='))
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
    fn legacy_id_first_and_paseo_prompt_first_share_their_existing_handlers() {
        let legacy = argv(&["vessel", "send", "ab12", "hello"]);
        assert_eq!(normalize(legacy.clone()), legacy);
        assert_eq!(
            normalize(argv(&["vessel", "send", "hello", "ab12"])),
            argv(&["vessel", "agent", "send", "hello", "ab12"])
        );
        assert_eq!(
            normalize(argv(&["vessel", "send", "ab12", "--prompt", "hello"])),
            argv(&["vessel", "agent", "send", "ab12", "--prompt", "hello"])
        );
    }
}
