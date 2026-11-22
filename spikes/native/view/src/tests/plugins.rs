//! Settings › Plugins (ADR 0115), after the page's `plugins.test.ts` and
//! Paseo's `plugins-page-state.ts` / `plugin-install-form-model.ts`: the
//! state order, the install field, the status looks and the words; then
//! the page's flow over its calls (the page's `use-plugins.ts`): what is
//! asked, which answers count, and the watch.

use chrono::{TimeZone as _, Timelike as _, Utc};
use vessel_proto::parking::ParkAfter;
use vessel_proto::plugins::{
    LogEntry, LogSequence, LogStream, LogTruncation, PluginGeneration, PluginId, PluginListItem,
    PluginLogs, PluginSourceKind, PluginStatus,
};
use vessel_proto::restart::RestartRecovery;
use vessel_proto::settings::{AgentDrivers, NodeSettings};
use vessel_proto::{ErrorCode, ProtocolVersion};

use crate::devices::{MachineKey, Tickets};
use crate::localization::{Locale, Translator};
use crate::plugins::{
    Action, Call, Change, Feedback, Listed, LogsView, Page, PageState, Pending, Problem, Reached,
    Reply, Request, RowLook, Tone, change_words, install_source, log_head, page_state,
    problem_words, source_line, speaks, status_tone, status_word,
};

const READY_MACHINE: Reached = Reached {
    connected: true,
    supported: true,
};
const READY_LIST: Listed = Listed {
    loading: false,
    error: false,
    count: 1,
};

fn id(text: &str) -> PluginId {
    text.parse().unwrap()
}

fn plugin(name: &str, enabled: bool, status: PluginStatus) -> PluginListItem {
    PluginListItem {
        id: id(name),
        name: None,
        description: None,
        icon: None,
        path: None,
        source: PluginSourceKind::Directory,
        enabled,
        status,
        error: None,
    }
}

fn settings(plugins_enabled: bool) -> NodeSettings {
    NodeSettings {
        restart_recovery: RestartRecovery::default(),
        drivers: AgentDrivers::default(),
        park_after: ParkAfter::Never,
        metadata: None,
        plugins_enabled,
        terminal_agent_hooks: false,
        terminal_profiles: None,
        vessel_tools: None,
    }
}

fn english() -> crate::localization::Plugins<'static> {
    Translator::get(Locale::from_tag("en").unwrap())
        .unwrap()
        .plugins()
}

fn korean() -> crate::localization::Plugins<'static> {
    Translator::get(Locale::from_tag("ko").unwrap())
        .unwrap()
        .plugins()
}

fn requests(calls: &[Call]) -> Vec<Request> {
    calls.iter().map(|call| call.request.clone()).collect()
}

/// A page for "laptop" whose first list, switch and watch have answered.
fn listed(tickets: &mut Tickets, plugins: Vec<PluginListItem>, on: bool) -> (Page, MachineKey) {
    let laptop = MachineKey::from_book("laptop");
    let (mut page, calls) = Page::open(laptop.clone(), tickets);
    let [list, switch, watch] = calls.as_slice() else {
        panic!("three first calls, not {calls:?}");
    };
    let answers = [
        (list.ticket, Reply::List(plugins)),
        (switch.ticket, Reply::Settings(settings(on))),
        (watch.ticket, Reply::Generation(PluginGeneration::new(3))),
    ];
    for (ticket, reply) in answers {
        let _answered = page.answer(tickets, &laptop, ticket, Ok(reply));
    }
    (page, laptop)
}

#[test]
fn checks_offline_then_support_loading_error_and_count_in_order() {
    let offline = Reached {
        connected: false,
        supported: false,
    };
    let old = Reached {
        connected: true,
        supported: false,
    };
    let loading = Listed {
        loading: true,
        error: true,
        ..READY_LIST
    };
    let failed = Listed {
        error: true,
        ..READY_LIST
    };
    let none = Listed {
        count: 0,
        ..READY_LIST
    };
    assert_eq!(page_state(offline, loading), PageState::Offline);
    assert_eq!(page_state(old, loading), PageState::Unsupported);
    assert_eq!(page_state(READY_MACHINE, loading), PageState::Loading);
    assert_eq!(page_state(READY_MACHINE, failed), PageState::Error);
    assert_eq!(page_state(READY_MACHINE, none), PageState::Empty);
    assert_eq!(page_state(READY_MACHINE, READY_LIST), PageState::Ready);
}

#[test]
fn needs_the_plugin_protocol() {
    // The generated constant, not a copy: the number is assigned at merge.
    let since = crate::plugins::PROTOCOL.unwrap();
    assert_eq!(Some(since), vessel_proto::plugins::PLUGINS_PROTOCOL);
    assert!(speaks(Some(since)));
    assert!(!speaks(Some(ProtocolVersion::new(
        since.get().saturating_sub(1)
    ))));
    assert!(!speaks(None));
}

#[test]
fn sends_only_a_trimmed_non_empty_source() {
    assert_eq!(install_source("   "), None);
    assert_eq!(install_source(""), None);
    assert_eq!(
        install_source("  ~/plugins/night "),
        Some("~/plugins/night")
    );
}

#[test]
fn gives_paseos_status_words_and_badge_tones() {
    assert_eq!(status_tone(PluginStatus::Running), Tone::Ok);
    assert_eq!(status_tone(PluginStatus::Failed), Tone::Danger);
    assert_eq!(status_tone(PluginStatus::Disabled), Tone::Neutral);
    let (en, ko) = (english(), korean());
    assert_eq!(status_word(en, PluginStatus::Running), "running");
    assert_eq!(status_word(en, PluginStatus::Failed), "failed");
    assert_eq!(status_word(en, PluginStatus::Disabled), "disabled");
    assert_eq!(status_word(ko, PluginStatus::Running), "실행 중");
    assert_eq!(status_word(ko, PluginStatus::Failed), "실패");
    assert_eq!(status_word(ko, PluginStatus::Disabled), "비활성화");
}

#[test]
fn says_each_change_as_paseos_feedback() {
    let (en, ko) = (english(), korean());
    let night = id("night");
    // Fluent isolates a value (U+2068 .. U+2069).
    let said = |change: Change| change_words(en, &change);
    assert_eq!(
        said(Change::Installed(night.clone())),
        "Installed \u{2068}night\u{2069}"
    );
    assert_eq!(
        said(Change::Acted(Action::Reload, night.clone())),
        "Reloaded \u{2068}night\u{2069}"
    );
    assert_eq!(
        said(Change::Acted(Action::Enable, night.clone())),
        "Enabled \u{2068}night\u{2069}"
    );
    assert_eq!(
        said(Change::Acted(Action::Disable, night.clone())),
        "Disabled \u{2068}night\u{2069}"
    );
    assert_eq!(
        said(Change::Acted(Action::Remove, night.clone())),
        "Removed \u{2068}night\u{2069}"
    );
    assert_eq!(
        said(Change::Kept(night.clone())),
        "Kept \u{2068}night\u{2069}"
    );
    assert_eq!(said(Change::Switched(true)), "Plugins enabled");
    assert_eq!(said(Change::Switched(false)), "Plugins disabled");
    assert_eq!(
        change_words(ko, &Change::Installed(night)),
        "\u{2068}night\u{2069} 설치됨"
    );
    assert_eq!(
        change_words(ko, &Change::Switched(false)),
        "플러그인 비활성화됨"
    );
}

#[test]
fn says_the_page_and_its_dialogs_in_both_languages() {
    let (en, ko) = (english(), korean());
    assert_eq!(en.trusted_title(), "Plugins are trusted code");
    assert_eq!(
        en.trusted_description(),
        "They run on this machine without sandboxing. Install only code you trust."
    );
    assert_eq!(en.global_title(), "Enable plugins");
    assert_eq!(
        en.global_hint(),
        "Global switch for every configured plugin"
    );
    assert_eq!(en.source_label(), "Plugin source");
    assert_eq!(
        en.source_placeholder(),
        "Directory on this machine, ~/… or /…"
    );
    assert_eq!(en.install(), "Install plugin");
    assert_eq!(en.installing(), "Installing…");
    assert_eq!(en.offline_title(), "Plugin host is offline");
    assert_eq!(
        en.offline_description(),
        "Reconnect to this machine to manage its plugins."
    );
    assert_eq!(en.update_title(), "Update this machine to manage plugins");
    assert_eq!(en.loading(), "Loading plugins…");
    assert_eq!(en.error_title(), "Unable to load plugins");
    assert_eq!(en.empty(), "No plugins configured");
    assert_eq!(
        en.remove_message(),
        "This removes the configuration. The source directory is not deleted."
    );
    assert_eq!(
        en.remove_title("night").unwrap(),
        "Remove \u{2068}night\u{2069}?"
    );
    assert_eq!(
        en.menu("night").unwrap(),
        "Actions for \u{2068}night\u{2069}"
    );
    assert_eq!(
        en.logs_title("night").unwrap(),
        "Logs: \u{2068}night\u{2069}"
    );
    assert_eq!(en.logs_empty(), "No plugin output yet");
    assert_eq!(en.logs_error_title(), "Unable to load plugin logs");
    assert_eq!(ko.trusted_title(), "플러그인은 신뢰하는 코드입니다");
    assert_eq!(ko.global_title(), "플러그인 활성화");
    assert_eq!(ko.install(), "플러그인 설치");
    assert_eq!(
        ko.update_title(),
        "플러그인을 관리하려면 이 머신을 업데이트하세요"
    );
    assert_eq!(
        ko.remove_title("night").unwrap(),
        "\u{2068}night\u{2069}\u{2068}를\u{2069} 제거할까요?"
    );
    assert_eq!(ko.menu("night").unwrap(), "\u{2068}night\u{2069} 작업");
}

#[test]
fn shows_the_source_only_when_the_node_gave_the_path() {
    let en = english();
    let mut night = plugin("night", true, PluginStatus::Running);
    assert_eq!(source_line(en, &night), None);
    night.path = Some("/home/a/night".to_owned());
    assert_eq!(
        source_line(en, &night).as_deref(),
        Some("Directory: \u{2068}/home/a/night\u{2069}")
    );
}

#[test]
fn shows_a_nodes_words_alone_and_otherwise_a_sentence() {
    let (en, ko) = (english(), korean());
    let worded = Problem::Refused {
        code: ErrorCode::InvalidRequest,
        message: Some("plugin manifest is missing".to_owned()),
    };
    assert_eq!(problem_words(en, &worded), "plugin manifest is missing");
    assert_eq!(problem_words(ko, &worded), "plugin manifest is missing");
    let coded = Problem::Refused {
        code: ErrorCode::Forbidden,
        message: None,
    };
    assert_eq!(
        problem_words(en, &coded),
        "The machine refused it (\u{2068}forbidden\u{2069})."
    );
    assert_eq!(
        problem_words(en, &Problem::Unsupported),
        "That machine runs an older Vessel; update it to use this."
    );
    assert_eq!(
        problem_words(ko, &Problem::Offline),
        "머신이 연결되어 있지 않습니다. 머신이 켜져 있고 Vessel이 실행 중인지 확인하세요."
    );
}

#[test]
fn heads_a_log_entry_with_its_time_and_where_it_came_from() {
    let entry = |stream| LogEntry {
        sequence: LogSequence::new(1),
        at: Utc.with_ymd_and_hms(2026, 11, 19, 18, 0, 0).unwrap(),
        stream,
        message: "ready".parse().unwrap(),
        truncation: LogTruncation::Complete,
    };
    assert_eq!(
        log_head(&entry(LogStream::Host)),
        "2026-11-19T18:00:00.000Z vessel"
    );
    assert_eq!(
        log_head(&entry(LogStream::Stderr)),
        "2026-11-19T18:00:00.000Z stderr"
    );
    let mut fine = entry(LogStream::Host);
    fine.at = Utc
        .with_ymd_and_hms(2026, 11, 20, 0, 46, 29)
        .unwrap()
        .with_nanosecond(163_814_811)
        .unwrap();
    assert_eq!(log_head(&fine), "2026-11-20T00:46:29.163Z vessel");
}

#[test]
fn opens_with_the_list_the_switch_and_a_watch_for_the_current_generation() {
    let mut tickets = Tickets::default();
    let (page, calls) = Page::open(MachineKey::from_book("laptop"), &mut tickets);
    assert_eq!(
        requests(&calls),
        [
            Request::List,
            Request::ReadSwitch,
            Request::Watch(PluginGeneration::new(u64::MAX)),
        ]
    );
    assert_eq!(page.state(), PageState::Loading);
    assert_eq!(page.enabled(), None);
}

#[test]
fn shows_the_list_and_ignores_another_machines_or_an_older_answer() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let (mut page, calls) = Page::open(laptop.clone(), &mut tickets);
    let list = calls.first().unwrap().ticket;
    let other = MachineKey::from_book("build-server");
    let foreign = page.answer(&mut tickets, &other, list, Ok(Reply::List(Vec::new())));
    assert!(!foreign.changed);
    assert_eq!(page.state(), PageState::Loading);
    let retry = page.retry(&mut tickets).unwrap();
    assert_eq!(retry.request, Request::List);
    // The first ask was replaced: its answer is dropped.
    let late = page.answer(&mut tickets, &laptop, list, Ok(Reply::List(Vec::new())));
    assert!(!late.changed);
    let failed = Problem::Refused {
        code: ErrorCode::Forbidden,
        message: None,
    };
    let answered = page.answer(&mut tickets, &laptop, retry.ticket, Err(failed.clone()));
    assert!(answered.changed);
    assert_eq!(page.state(), PageState::Error);
    assert_eq!(page.list_problem(), Some(&failed));
}

#[test]
fn switches_plugins_once_at_a_time_and_reads_again_after() {
    let mut tickets = Tickets::default();
    let (mut page, laptop) = listed(&mut tickets, Vec::new(), false);
    assert_eq!(page.state(), PageState::Empty);
    assert_eq!(page.enabled(), Some(false));
    let on = page.toggle(&mut tickets, true).unwrap();
    assert_eq!(on.request, Request::Switch(true));
    assert_eq!(page.pending(), Some(&Pending::Switch(true)));
    assert!(page.toggle(&mut tickets, true).is_none());
    assert!(page.install(&mut tickets, "~/night").is_none());
    let answered = page.answer(
        &mut tickets,
        &laptop,
        on.ticket,
        Ok(Reply::Settings(settings(true))),
    );
    assert_eq!(
        requests(&answered.calls),
        [Request::List, Request::ReadSwitch]
    );
    assert_eq!(page.enabled(), Some(true));
    assert!(!page.busy());
    assert_eq!(
        page.feedback(),
        Some(&Feedback::Done(Change::Switched(true)))
    );
}

#[test]
fn installs_a_trimmed_source_and_empties_the_field_only_on_success() {
    let mut tickets = Tickets::default();
    let (mut page, laptop) = listed(&mut tickets, Vec::new(), true);
    assert!(page.install(&mut tickets, "  ").is_none());
    let install = page.install(&mut tickets, " ~/plugins/night ").unwrap();
    assert_eq!(
        install.request,
        Request::Install("~/plugins/night".to_owned())
    );
    assert_eq!(page.pending(), Some(&Pending::Install));
    let refused = Problem::Refused {
        code: ErrorCode::InvalidRequest,
        message: Some("no vessel-plugin.json".to_owned()),
    };
    let answered = page.answer(&mut tickets, &laptop, install.ticket, Err(refused.clone()));
    assert!(!answered.installed);
    assert_eq!(page.feedback(), Some(&Feedback::Failed(refused)));
    let again = page.install(&mut tickets, "~/plugins/night").unwrap();
    // A new change clears the last note (Paseo's `setFeedback(null)`).
    assert_eq!(page.feedback(), None);
    let item = plugin("night", true, PluginStatus::Running);
    let answered = page.answer(&mut tickets, &laptop, again.ticket, Ok(Reply::Item(item)));
    assert!(answered.installed);
    assert_eq!(
        page.feedback(),
        Some(&Feedback::Done(Change::Installed(id("night"))))
    );
}

#[test]
fn reloads_only_an_enabled_plugin_and_says_which_row_waits() {
    let mut tickets = Tickets::default();
    let plugins = vec![
        plugin("night", true, PluginStatus::Running),
        plugin("broken", false, PluginStatus::Disabled),
    ];
    let (mut page, laptop) = listed(&mut tickets, plugins, true);
    assert_eq!(page.state(), PageState::Ready);
    assert!(
        page.act(&mut tickets, Action::Reload, &id("broken"))
            .is_none()
    );
    assert!(
        page.act(&mut tickets, Action::Reload, &id("gone"))
            .is_none()
    );
    let reload = page
        .act(&mut tickets, Action::Reload, &id("night"))
        .unwrap();
    assert_eq!(reload.request, Request::Act(Action::Reload, id("night")));
    assert_eq!(
        page.row(&id("night")),
        RowLook {
            busy: true,
            mine: Some(Action::Reload)
        }
    );
    assert_eq!(
        page.row(&id("broken")),
        RowLook {
            busy: true,
            mine: None
        }
    );
    let _answered = page.answer(&mut tickets, &laptop, reload.ticket, Ok(Reply::Done));
    assert_eq!(
        page.feedback(),
        Some(&Feedback::Done(Change::Acted(Action::Reload, id("night"))))
    );
    let enable = page
        .act(&mut tickets, Action::Enable, &id("broken"))
        .unwrap();
    assert_eq!(enable.request, Request::Act(Action::Enable, id("broken")));
}

#[test]
fn removes_only_after_asking_and_keeps_on_cancel() {
    let mut tickets = Tickets::default();
    let plugins = vec![plugin("night", true, PluginStatus::Running)];
    let (mut page, laptop) = listed(&mut tickets, plugins, true);
    assert!(
        page.act(&mut tickets, Action::Remove, &id("night"))
            .is_none()
    );
    assert_eq!(page.removing(), Some(&id("night")));
    page.keep();
    assert_eq!(page.removing(), None);
    assert_eq!(
        page.feedback(),
        Some(&Feedback::Done(Change::Kept(id("night"))))
    );
    assert!(
        page.act(&mut tickets, Action::Remove, &id("night"))
            .is_none()
    );
    let remove = page.remove(&mut tickets).unwrap();
    assert_eq!(remove.request, Request::Act(Action::Remove, id("night")));
    assert_eq!(page.row(&id("night")).mine, Some(Action::Remove));
    let _answered = page.answer(&mut tickets, &laptop, remove.ticket, Ok(Reply::Done));
    assert_eq!(
        page.feedback(),
        Some(&Feedback::Done(Change::Acted(Action::Remove, id("night"))))
    );
    assert!(page.remove(&mut tickets).is_none());
}

#[test]
fn lists_again_when_the_generation_moves_and_stops_on_a_failure() {
    let mut tickets = Tickets::default();
    let laptop = MachineKey::from_book("laptop");
    let (mut page, calls) = Page::open(laptop.clone(), &mut tickets);
    let watch = calls.last().unwrap().ticket;
    let first = page.answer(
        &mut tickets,
        &laptop,
        watch,
        Ok(Reply::Generation(PluginGeneration::new(3))),
    );
    assert!(!first.changed);
    assert_eq!(
        requests(&first.calls),
        [Request::Watch(PluginGeneration::new(3))]
    );
    let same = page.answer(
        &mut tickets,
        &laptop,
        first.calls.first().unwrap().ticket,
        Ok(Reply::Generation(PluginGeneration::new(3))),
    );
    assert!(!same.changed);
    assert_eq!(
        requests(&same.calls),
        [Request::Watch(PluginGeneration::new(3))]
    );
    let moved = page.answer(
        &mut tickets,
        &laptop,
        same.calls.first().unwrap().ticket,
        Ok(Reply::Generation(PluginGeneration::new(4))),
    );
    assert!(moved.changed);
    assert_eq!(
        requests(&moved.calls),
        [
            Request::List,
            Request::ReadSwitch,
            Request::Watch(PluginGeneration::new(4)),
        ]
    );
    let stopped = page.answer(
        &mut tickets,
        &laptop,
        moved.calls.last().unwrap().ticket,
        Err(Problem::Dropped),
    );
    assert!(stopped.calls.is_empty());
}

#[test]
fn reads_logs_on_open_and_refresh_keeping_the_last_answer_meanwhile() {
    let mut tickets = Tickets::default();
    let plugins = vec![plugin("night", true, PluginStatus::Running)];
    let (mut page, laptop) = listed(&mut tickets, plugins, true);
    let open = page.open_logs(&mut tickets, &id("night")).unwrap();
    assert_eq!(open.request, Request::Logs(id("night")));
    let logs = page.logs().unwrap();
    assert_eq!(logs.view(), LogsView::Loading);
    assert!(logs.refreshing());
    assert!(page.refresh_logs(&mut tickets).is_none());
    let empty = PluginLogs {
        entries: Vec::new(),
    };
    let _answered = page.answer(&mut tickets, &laptop, open.ticket, Ok(Reply::Logs(empty)));
    assert_eq!(page.logs().unwrap().view(), LogsView::Empty);
    let refresh = page.refresh_logs(&mut tickets).unwrap();
    assert_eq!(page.logs().unwrap().view(), LogsView::Empty);
    let _answered = page.answer(
        &mut tickets,
        &laptop,
        refresh.ticket,
        Err(Problem::TimedOut),
    );
    assert_eq!(
        page.logs().unwrap().view(),
        LogsView::Failed(&Problem::TimedOut)
    );
    page.close_logs();
    assert!(page.logs().is_none());
}
