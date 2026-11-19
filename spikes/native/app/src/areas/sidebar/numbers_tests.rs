//! The rows' shortcut numbers (TRACKER 663, Paseo's shortcut badge): while
//! the jump modifier is held, the first nine rows in the order the keys
//! walk are numbered from 1; otherwise none is.
use gpui::TestAppContext;
use vessel_app_view::prefs::Grouping;
use vessel_app_view::{Card, Fleet, Machine, Reach};
use vessel_proto::{HarnessKind, SessionId, SessionInfo, SessionMode, SessionSpec, SessionStatus};

use super::Numbers;
use crate::shell::{Shell, language_tests};

fn machine(name: &str, count: u16) -> Machine {
    let sessions = (0..count)
        .map(|index| {
            // Newer first: the sidebar lists the last created at the top.
            let at = chrono::DateTime::from_timestamp(i64::from(index), 0).unwrap();
            let id: SessionId = format!("{index:08x}-0000-4000-8000-0000000000{name}")
                .parse()
                .unwrap();
            let spec = SessionSpec::new(
                HarnessKind::ClaudeCode,
                SessionMode::Structured {
                    model: None,
                    resume: None,
                    options: Vec::new(),
                },
            );
            let mut info = SessionInfo::new(id, spec, at);
            info.status = SessionStatus::Running;
            Card::from_session(name, &info)
        })
        .collect();
    Machine {
        connection_epoch: None,
        node: None,
        name: name.to_owned(),
        reach: Reach::Online,
        sessions,
        agents: Vec::new(),
        protocol: Some(vessel_proto::PROTOCOL_VERSION),
        version: None,
    }
}

#[gpui::test]
fn the_first_nine_rows_the_keys_walk_are_numbered_while_held(cx: &mut TestAppContext) {
    language_tests::init(cx);
    let (shell, cx) = cx.add_window_view(Shell::without_io);
    shell.update(cx, |shell, _| {
        shell.fleet = Fleet {
            machines: vec![machine("aa", 4), machine("bb", 8)],
            read_at: None,
        };
        shell.prefs.settings.sidebar.group = Grouping::Machine;
    });
    shell.read_with(cx, |shell, _| {
        let order: Vec<Card> = shell
            .fleet
            .sidebar_order(shell.prefs.settings.sidebar)
            .into_iter()
            .cloned()
            .collect();
        // Four of the first machine, five of the second (its first five).
        assert_eq!(order.len(), 9);
        let held = Numbers::of_shell(shell, true);
        for (index, card) in order.iter().enumerate() {
            assert_eq!(held.of(card), index.checked_add(1));
        }
        let released = Numbers::of_shell(shell, false);
        assert!(order.iter().all(|card| released.of(card).is_none()));
        // A row past the ninth has no number.
        let beyond = shell
            .fleet
            .machines
            .iter()
            .flat_map(|machine| &machine.sessions)
            .find(|card| !order.iter().any(|listed| listed.session == card.session));
        assert_eq!(beyond.and_then(|card| held.of(card)), None);
    });
}
