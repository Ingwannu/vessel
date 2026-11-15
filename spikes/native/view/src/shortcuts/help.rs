//! Search projection for the help dialog and Settings. Rows carry the effective
//! binding used by the engine, so rebinding immediately changes every key alias.
//! Alias expansion is per chord, never across the complete multi-step sequence.

use super::sequence::KeySequence;
use super::{Chord, Platform, ShortcutGroup, chord_caps};

/// Display the source's default question mark as `?`. A user override retains
/// its explicit modifiers; every other action uses the shared chord caps.
#[must_use]
pub fn display_caps(
    action: super::Action,
    sequence: &KeySequence,
    overridden: bool,
    platform: Platform,
) -> Vec<Vec<String>> {
    if action == super::Action::Help && !overridden {
        return vec![vec!["?".to_owned()]];
    }
    sequence
        .chords()
        .map(|chord| chord_caps(chord, platform))
        .collect()
}

/// Translated row presentation, prepared from one current registry snapshot.
#[derive(Debug, Clone)]
pub struct HelpRow<'a, A> {
    /// Typed action or binding identity used by the caller.
    pub action: A,
    /// Translated action label.
    pub label: &'a str,
    /// Optional translated availability note.
    pub note: Option<&'a str>,
    /// Effective sequence; None explicitly displays Unassigned.
    pub keys: Option<&'a KeySequence>,
}

/// Sections keep reference ordering even when a search leaves only a few rows.
#[derive(Debug, Clone)]
pub struct HelpSection<'a, A> {
    /// The one registry's help group.
    pub group: ShortcutGroup,
    /// Translated heading, searchable independently of row text.
    pub title: &'a str,
    /// Deduplicated, platform-filtered rows in help order.
    pub rows: Vec<HelpRow<'a, A>>,
}

/// Narrow by translated label, note, displayed caps and familiar key spellings.
/// A matching section title retains every row, including unassigned actions.
#[must_use]
pub fn filter_help<'a, A: Clone>(
    sections: &[HelpSection<'a, A>],
    query: &str,
    platform: Platform,
) -> Vec<HelpSection<'a, A>> {
    let query = query
        .trim_matches(crate::matcher::whitespace)
        .to_lowercase();
    sections
        .iter()
        .filter_map(|section| {
            if query.is_empty() || section.title.to_lowercase().contains(&query) {
                return Some(section.clone());
            }
            let rows: Vec<_> = section
                .rows
                .iter()
                .filter(|row| {
                    let keys = row
                        .keys
                        .map(|keys| search_keys(keys, platform))
                        .unwrap_or_default();
                    [row.label, row.note.unwrap_or_default(), &keys]
                        .join(" ")
                        .to_lowercase()
                        .contains(&query)
                })
                .cloned()
                .collect();
            (!rows.is_empty()).then(|| HelpSection {
                group: section.group,
                title: section.title,
                rows,
            })
        })
        .collect()
}

/// Per-chord synonyms include both space and plus notation; the rendered glyphs
/// remain searchable too. No alias invents a modifier combination across steps.
#[must_use]
pub fn search_keys(sequence: &KeySequence, platform: Platform) -> String {
    sequence
        .chords()
        .map(|chord| aliases(chord, platform))
        .collect::<Vec<_>>()
        .join(" ")
}

fn aliases(chord: &Chord, platform: Platform) -> String {
    let mut choices: Vec<&[&str]> = Vec::new();
    match platform {
        Platform::Mac => {
            if chord.ctrl {
                choices.push(&["ctrl", "control"]);
            }
            if chord.alt {
                choices.push(&["alt", "option"]);
            }
            if chord.shift {
                choices.push(&["shift"]);
            }
            if chord.primary || chord.meta {
                choices.push(&["cmd", "command"]);
            }
        }
        Platform::Other => {
            if chord.primary || chord.ctrl {
                choices.push(&["ctrl", "control"]);
            }
            if chord.alt {
                choices.push(&["alt", "option"]);
            }
            if chord.shift {
                choices.push(&["shift"]);
            }
            if chord.meta {
                choices.push(&["win", "windows"]);
            }
        }
    }
    let mut combinations: Vec<Vec<&str>> = vec![Vec::new()];
    for variants in choices {
        combinations = combinations
            .into_iter()
            .flat_map(|prefix| {
                variants.iter().map(move |variant| {
                    let mut expanded = prefix.clone();
                    expanded.push(variant);
                    expanded
                })
            })
            .collect();
    }
    for combination in &mut combinations {
        combination.push(&chord.key);
    }
    let caps = chord_caps(chord, platform);
    combinations
        .into_iter()
        .flat_map(|combination| [combination.join(" "), combination.join("+")])
        .chain([caps.join(" "), caps.join("")])
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_matches_labels_notes_section_titles_and_both_modifier_spellings() {
        let keys = KeySequence::parse("Mod+K").unwrap();
        let sections = [HelpSection {
            group: ShortcutGroup::General,
            title: "General",
            rows: vec![
                HelpRow {
                    action: 1,
                    label: "Command center",
                    note: Some("Outside a text field"),
                    keys: Some(&keys),
                },
                HelpRow {
                    action: 2,
                    label: "Unassigned action",
                    note: None,
                    keys: None,
                },
            ],
        }];
        for query in [
            "center",
            "TEXT FIELD",
            "cmd k",
            "command+k",
            "⌘K",
            "\u{feff}center\u{feff}",
        ] {
            assert_eq!(
                filter_help(&sections, query, Platform::Mac)[0].rows[0].action,
                1,
                "{query}"
            );
        }
        assert_eq!(
            filter_help(&sections, "general", Platform::Mac)[0]
                .rows
                .len(),
            2
        );
        assert_eq!(
            filter_help(&sections, "control+k", Platform::Other)[0]
                .rows
                .len(),
            1
        );
        assert!(filter_help(&sections, "command+x", Platform::Mac).is_empty());
        assert_eq!(
            filter_help(&sections, "\u{feff}", Platform::Mac)[0]
                .rows
                .len(),
            2
        );
        assert!(filter_help(&sections, "\u{85}center\u{85}", Platform::Mac).is_empty());
    }

    #[test]
    fn chord_aliases_do_not_flatten_two_shortcuts_into_one() {
        let sequence = KeySequence::parse("Mod+K Mod+S").unwrap();
        let aliases = search_keys(&sequence, Platform::Mac).to_lowercase();
        assert!(aliases.contains("command+k"));
        assert!(aliases.contains("cmd s"));
        assert!(!aliases.contains("command+k+command+s"));
    }
}
