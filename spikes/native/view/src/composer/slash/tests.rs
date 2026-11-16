//! Paseo's cases (`utils/agent-command-autocomplete.test.ts`,
//! `components/ui/autocomplete-utils.test.ts`) where they apply at the
//! prompt's start, with the shapes the fake fleet's agents send.

use vessel_proto::AgentCommand;

use super::*;

fn command(name: &str) -> AgentCommand {
    AgentCommand {
        name: name.parse().unwrap(),
        description: format!("What {name} does"),
        hint: None,
        kind: None,
    }
}

fn names(commands: &[&AgentCommand]) -> Vec<String> {
    commands
        .iter()
        .map(|command| command.name.as_str().to_owned())
        .collect()
}

#[test]
fn a_command_is_typed_only_from_the_prompts_start_to_the_caret() {
    assert_eq!(
        active("/rew", 4),
        Some(SlashRange {
            start: 0,
            end: 4,
            query: "rew".to_owned()
        })
    );
    assert_eq!(
        active("/", 1),
        Some(SlashRange {
            start: 0,
            end: 1,
            query: String::new()
        })
    );
    assert_eq!(
        active("/review the rest", 3).map(|range| range.query),
        Some("re".to_owned()),
        "the caret, not the word's end, ends the query"
    );
    assert_eq!(active("use /tas", 8), None, "not at the start");
    assert_eq!(active("/taste now", 10), None, "past a space");
    assert_eq!(active("/tmp/project", 12), None, "a path");
    assert_eq!(active("/say \"hi", 8), None);
    assert_eq!(active("/rew", 0), None, "the caret before the slash");
    assert_eq!(active("plain", 5), None);
}

#[test]
fn a_caret_inside_a_character_counts_from_its_start() {
    // `/` is one byte, 한 the next three (1..4), 글 three more (4..7).
    assert_eq!(
        active("/한글", 4).map(|range| range.query),
        Some("한".to_owned())
    );
    assert_eq!(
        active("/한글", 3).map(|range| (range.end, range.query)),
        Some((1, String::new())),
        "a caret inside 한 counts from 한's start"
    );
}

#[test]
fn choosing_replaces_the_typed_name_and_spaces_at_the_end() {
    let range = active("/tas", 4).unwrap();
    assert_eq!(
        apply("/tas", &range, "taste"),
        Applied {
            text: "/taste ".to_owned(),
            caret: 7
        }
    );
    let range = active("/tas more", 4).unwrap();
    assert_eq!(
        apply("/tas more", &range, "taste"),
        Applied {
            text: "/taste more".to_owned(),
            caret: 6
        },
        "what follows the caret stays, with no added space"
    );
}

#[test]
fn name_prefixes_rank_above_later_word_starts() {
    let commands = [
        command("paseo-committee"),
        command("commit"),
        command("paseo-advisor"),
    ];
    assert_eq!(
        names(&ranked(&commands, "comm")),
        ["commit", "paseo-committee"]
    );
}

#[test]
fn an_empty_query_keeps_the_agents_order() {
    let commands = [command("review"), command("compact"), command("init")];
    assert_eq!(names(&ranked(&commands, "")), ["review", "compact", "init"]);
    assert_eq!(
        names(&ranked(&commands, "  ")),
        ["review", "compact", "init"]
    );
}

#[test]
fn ties_are_broken_by_name_and_descriptions_are_not_searched() {
    // Both match "run" at a word's start five characters in: a tie.
    let commands = [command("zeta-run"), command("beta-run"), command("other")];
    assert_eq!(names(&ranked(&commands, "run")), ["beta-run", "zeta-run"]);
    // An earlier match ranks first whatever the names (Paseo's
    // `compareMatchScores`: tier, then where it starts).
    let commands = [command("alpha-run"), command("zeta-run")];
    assert_eq!(names(&ranked(&commands, "run")), ["zeta-run", "alpha-run"]);
    assert!(ranked(&commands, "does").is_empty());
}

#[test]
fn the_best_row_is_drawn_nearest_the_text() {
    assert_eq!(
        drawn(vec!["alpha", "beta", "gamma"]),
        ["gamma", "beta", "alpha"]
    );
}

#[test]
fn the_nearest_row_is_chosen_on_opening_and_when_the_query_changes() {
    let mut chosen = Chosen::default();
    assert_eq!(chosen.follow("", 3), Some(2));
    assert_eq!(chosen.step(Step::Up, 3), Some(1));
    assert_eq!(chosen.follow("", 3), Some(1), "kept while the query holds");
    assert_eq!(chosen.follow("c", 2), Some(1), "back to the nearest");
    assert_eq!(chosen.follow("co", 0), None, "no rows, none chosen");
    assert_eq!(chosen.taken(0), None);
}

#[test]
fn arrows_wrap_and_enter_takes_the_chosen_or_the_nearest() {
    let mut chosen = Chosen::default();
    assert_eq!(chosen.step(Step::Down, 3), Some(0));
    assert_eq!(chosen.step(Step::Up, 3), Some(2));
    assert_eq!(chosen.step(Step::Down, 3), Some(0));
    chosen.reset();
    assert_eq!(chosen.step(Step::Up, 3), Some(2));
    assert_eq!(chosen.taken(3), Some(2));
    chosen.point(0);
    assert_eq!(chosen.taken(3), Some(0));
    assert_eq!(chosen.taken(0), None);
    chosen.reset();
    assert_eq!(chosen.taken(4), Some(3));
}

#[test]
fn the_menu_lists_the_answering_commands_best_last_with_the_nearest_chosen() {
    let commands = [
        command("paseo-committee"),
        command("commit"),
        command("paseo-advisor"),
    ];
    let menu = Menu::of("/comm", 5, Some(&commands), &Chosen::default()).unwrap();
    assert_eq!(names(&menu.rows), ["paseo-committee", "commit"]);
    assert_eq!(menu.chosen, Some(1), "the best, nearest the text");
    assert_eq!(
        menu.taken(&Chosen::default())
            .map(|command| command.name.as_str()),
        Some("commit")
    );
}

#[test]
fn a_menu_without_commands_has_no_rows_and_none_outside_a_name() {
    let none = Menu::of("/", 1, None, &Chosen::default()).unwrap();
    assert!(none.rows.is_empty(), "the agent has not said");
    assert_eq!(none.chosen, None);
    assert_eq!(none.taken(&Chosen::default()), None);
    let empty: [AgentCommand; 0] = [];
    assert!(
        Menu::of("/x", 2, Some(&empty), &Chosen::default())
            .unwrap()
            .rows
            .is_empty()
    );
    assert_eq!(Menu::of("hello /x", 8, None, &Chosen::default()), None);
}

#[test]
fn a_moved_choice_is_kept_while_the_name_typed_holds() {
    let commands = [command("init"), command("review"), command("compact")];
    let mut chosen = Chosen::default();
    let menu = Menu::of("/", 1, Some(&commands), &chosen).unwrap();
    assert_eq!(chosen.follow(&menu.range.query, menu.rows.len()), Some(2));
    assert_eq!(chosen.step(Step::Up, menu.rows.len()), Some(1));
    let menu = Menu::of("/", 1, Some(&commands), &chosen).unwrap();
    assert_eq!(menu.chosen, Some(1));
    assert_eq!(
        menu.taken(&chosen).map(|command| command.name.as_str()),
        Some("review"),
        "drawn order is compact, review, init: review is the middle row"
    );
}
