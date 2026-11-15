//! Source precedence, inline restrictions, alias ranking and Unicode caret regressions.

use super::*;

fn provider(name: &str) -> AgentCommand {
    AgentCommand {
        name: name.parse().unwrap(),
        description: "description".to_owned(),
        hint: None,
        kind: None,
    }
}

#[test]
fn merge_reserves_builtins_aliases_and_first_plugin_before_provider_names() {
    let names: Vec<CommandName> = ["exit", "q", "new", "plug", "plug", "other"]
        .into_iter()
        .map(|name| name.parse().unwrap())
        .collect();
    let commands: Vec<_> = ["exit", "q", "new", "plug", "native"]
        .into_iter()
        .map(provider)
        .collect();
    let addresses: Vec<_> = (0..names.len()).collect();
    for target in [Target::Session, Target::Draft] {
        let entries = merge(
            target,
            names
                .iter()
                .zip(&addresses)
                .map(|(name, address)| Entry::plugin(name, address)),
            commands
                .iter()
                .map(|command| Entry::provider(command, Inline::RootOnly)),
        );
        let actual: Vec<_> = entries.iter().map(Entry::name).collect();
        assert_eq!(
            actual,
            if target == Target::Session {
                vec!["exit", "clear", "plug", "other", "native"]
            } else {
                vec!["plug", "other", "native"]
            }
        );
    }
}

#[test]
fn inline_offers_only_provider_skills_and_aliases_rank_the_canonical_builtin() {
    let plugin_name: CommandName = "skill-plugin".parse().unwrap();
    let commands = [provider("skill-provider"), provider("root-command")];
    let entries = merge(
        Target::Session,
        [Entry::plugin(&plugin_name, &())],
        [
            Entry::provider(&commands[0], Inline::Skill),
            Entry::provider(&commands[1], Inline::RootOnly),
        ],
    );
    let inline: Vec<_> = ranked(&entries, "", Position::Inline)
        .into_iter()
        .map(Entry::name)
        .collect();
    assert_eq!(inline, ["skill-provider"]);
    assert_eq!(ranked(&entries, "quit", Position::Start)[0].name(), "exit");
    assert!(ranked(&entries, "description", Position::Start).is_empty());
}

#[test]
fn nearest_valid_slash_ignores_paths_quotes_whitespace_and_inside_character_carets() {
    for (text, caret, start, query, position) in [
        ("/rev", 4, 0, "rev", Position::Start),
        ("use /rev", 8, 4, "rev", Position::Inline),
        ("/old words /새", 16, 11, "새", Position::Inline),
        ("/한글", 3, 0, "", Position::Start),
    ] {
        let found = active(text, caret).unwrap();
        assert_eq!(
            (
                found.range.start,
                found.range.query.as_str(),
                found.position
            ),
            (start, query, position),
            "{text}"
        );
    }
    for text in ["/tmp/file", "a/rev", "/rev arg", "use /'q", "plain"] {
        assert!(active(text, text.len()).is_none(), "{text}");
    }
}

#[test]
fn plugin_arguments_are_trimmed_once_and_attachments_disable_local_execution() {
    let name: CommandName = "plug".parse().unwrap();
    let address = 7;
    let entries = merge(Target::Session, [Entry::plugin(&name, &address)], []);
    let (chosen, args) =
        plugin_call("  /plug  한글 \n second  ", Context::TextOnly, &entries).unwrap();
    assert_eq!(*chosen, address);
    assert_eq!(args.as_str(), "한글 \n second");
    assert!(plugin_call("/plug", Context::Attached, &entries).is_none());
    assert!(plugin_call("/exit", Context::TextOnly, &entries).is_none());
}

#[test]
fn shared_javascript_whitespace_keeps_plugin_and_inline_boundaries_exact() {
    let name: CommandName = "plug".parse().unwrap();
    let entries = merge(Target::Session, [Entry::plugin(&name, &())], []);
    let ((), args) = plugin_call(
        "\u{feff}/plug\u{feff} hello\u{feff}",
        Context::TextOnly,
        &entries,
    )
    .unwrap();
    assert_eq!(args.as_str(), "hello");
    assert!(plugin_call("/plug\u{0085}hello", Context::TextOnly, &entries).is_none());
    let bom = "use\u{feff}/skill";
    assert_eq!(active(bom, bom.len()).unwrap().range.query, "skill");
    let nel = "use\u{0085}/skill";
    assert!(active(nel, nel.len()).is_none());
}
