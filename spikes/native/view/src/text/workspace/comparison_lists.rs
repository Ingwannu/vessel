//! Existing list-comparison wording, extracted without changing its cases.
use crate::workspace::command_changes::{Change, CommandLine};

/// "setup commands: 1 added and 1 changed", or `None` when the list is the
/// same.
pub(super) fn en(name: &str, list: &[CommandLine]) -> Option<String> {
    let count = |wanted: &str| {
        list.iter()
            .filter(|line| match line {
                CommandLine::Line(line) => match line.change {
                    Change::Added => wanted == "added",
                    Change::Removed => wanted == "removed",
                    Change::Same => false,
                },
                CommandLine::Changed { .. } => wanted == "changed",
            })
            .count()
    };
    let said: Vec<String> = ["added", "changed", "removed"]
        .into_iter()
        .filter_map(|change| {
            let count = count(change);
            (count > 0).then(|| format!("{count} {change}"))
        })
        .collect();
    (!said.is_empty()).then(|| format!("{name} commands: {}", said.join(" and ")))
}

pub(super) fn ko(name: &str, list: &[CommandLine]) -> Option<String> {
    let mut added = 0_usize;
    let mut changed = 0_usize;
    let mut removed = 0_usize;
    for line in list {
        match line {
            CommandLine::Line(line) => match line.change {
                Change::Added => added = added.saturating_add(1),
                Change::Removed => removed = removed.saturating_add(1),
                Change::Same => {}
            },
            CommandLine::Changed { .. } => changed = changed.saturating_add(1),
        }
    }
    let said: Vec<String> = [(added, "추가"), (changed, "변경"), (removed, "삭제")]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, word)| format!("{count}개 {word}"))
        .collect();
    (!said.is_empty()).then(|| format!("{name} 명령 {}", said.join(", ")))
}
