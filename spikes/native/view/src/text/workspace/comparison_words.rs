//! Comparison prose is extracted so legacy language modules can shrink while
//! new executable fields acquire complete typed summaries.
use super::{En, Ko, Local, WorkspaceWords, comparison_lists};
use crate::localization::{ENGLISH, Locale, Translator};
use crate::workspace::allocation_changes::AllocationLine;
use crate::workspace::command_changes::{Compared, ComparedWith};
use crate::workspace::terminal_changes::TerminalLine;

fn terminal_part(language: crate::Lang, terminals: &[TerminalLine]) -> Option<String> {
    if !terminals.iter().any(TerminalLine::changed) {
        return None;
    }
    let locale = match language {
        crate::Lang::En => ENGLISH,
        crate::Lang::Ko => Locale::from_tag("ko").unwrap_or(ENGLISH),
    };
    Some(Translator::get(locale).map_or_else(
        |_invalid_catalog| match language {
            crate::Lang::En => En.labels().approval_unseen.to_owned(),
            crate::Lang::Ko => Ko.labels().approval_unseen.to_owned(),
        },
        |translator| translator.scripts().terminal_changes().to_owned(),
    ))
}

pub(super) fn en(words: En, compared: &Compared, local: Local) -> String {
    let Compared::Changed {
        against,
        setup,
        teardown,
        scripts,
        terminals,
        allocation,
        unshown: _,
    } = compared
    else {
        return "No earlier approval of this project's commands is known, so every command is shown as it is.".to_owned();
    };
    let parts: Vec<String> = [
        comparison_lists::en("setup", setup),
        comparison_lists::en("teardown", teardown),
    ]
    .into_iter()
    .chain(scripts.iter().map(|line| words.script_part(line)))
    .chain(terminal_part(crate::Lang::En, terminals).map(Some))
    .chain(allocation_part(crate::Lang::En, allocation).map(Some))
    .flatten()
    .collect();
    let changed = if parts.is_empty() {
        "nothing that runs reads differently".to_owned()
    } else {
        parts.join("; ")
    };
    match against {
        ComparedWith::LastApproved { at } => {
            let (today, time) = local.when(*at);
            let when = if today { format!("today {time}") } else { time };
            format!("Since the set approved {when}: {changed}.")
        }
        ComparedWith::Folder => format!(
            "Against the set approved for the project's folder (this machine does not say which set was approved last): {changed}."
        ),
    }
}

pub(super) fn ko(words: Ko, compared: &Compared, local: Local) -> String {
    let Compared::Changed {
        against,
        setup,
        teardown,
        scripts,
        terminals,
        allocation,
        unshown: _,
    } = compared
    else {
        return "이 프로젝트의 명령을 승인한 기록이 없어서 모든 명령을 있는 그대로 보여 줍니다."
            .to_owned();
    };
    let parts: Vec<String> = [
        comparison_lists::ko("셋업", setup),
        comparison_lists::ko("정리", teardown),
    ]
    .into_iter()
    .chain(scripts.iter().map(|line| words.script_part(line)))
    .chain(terminal_part(crate::Lang::Ko, terminals).map(Some))
    .chain(allocation_part(crate::Lang::Ko, allocation).map(Some))
    .flatten()
    .collect();
    let changed = if parts.is_empty() {
        "실행되는 것 중 달라진 것은 없습니다".to_owned()
    } else {
        parts.join("; ")
    };
    match against {
        ComparedWith::LastApproved { at } => {
            let (today, time) = local.when(*at);
            let when = if today {
                format!("오늘 {time}")
            } else {
                time
            };
            format!("{when}에 승인한 세트와 비교: {changed}.")
        }
        ComparedWith::Folder => format!(
            "프로젝트 폴더에 승인된 세트와 비교(이 머신은 마지막으로 승인된 세트를 알려 주지 않습니다): {changed}."
        ),
    }
}

fn allocation_part(language: crate::Lang, allocation: &AllocationLine) -> Option<String> {
    if !allocation.changed() {
        return None;
    }
    let locale = match language {
        crate::Lang::En => ENGLISH,
        crate::Lang::Ko => Locale::from_tag("ko").unwrap_or(ENGLISH),
    };
    Some(Translator::get(locale).map_or_else(
        |_invalid_catalog| match language {
            crate::Lang::En => En.labels().approval_unseen.to_owned(),
            crate::Lang::Ko => Ko.labels().approval_unseen.to_owned(),
        },
        |translator| translator.approval().allocation_changes().to_owned(),
    ))
}
