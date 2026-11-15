//! Real shared-kit paint with typed observations; no transport or process exists.
use super::*;
use crate::shell::language_tests;
use gpui::{AppContext as _, Modifiers, Render, TestAppContext};
use vessel_app_kit::ui::{Entity, IntoElement};
use vessel_app_kit::{Scheme, Tokens};
use vessel_app_view::connection::{ConnectionEpochs, MachineKey};
use vessel_app_view::localization::{Locale, Translator};
use vessel_app_view::workspace::setup_observation::{Address, Support};
use vessel_proto::workspace::{
    SetupCommandIndex, SetupCommandProgress, SetupCommandResult, SetupCommandState, SetupCursor,
    SetupProgress, SetupProgressReply, SetupRunState,
};
use vessel_proto::{DurationMs, NodeId, SetupRevision, SetupRunId, TerminalBytes, WorktreeId};

struct Probe {
    shell: Entity<Shell>,
    model: Model,
    scheme: Scheme,
}
impl Render for Probe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        self.shell.update(cx, |shell, cx| {
            draw::render(
                &draw::Card {
                    shell,
                    model: &self.model,
                    dispatch: None,
                    tokens: &Tokens::of(self.scheme),
                },
                window,
                cx,
            )
        })
    }
}
fn fixture(result: Option<SetupCommandResult>) -> Model {
    let address = Address {
        context: ConnectionEpochs::default()
            .admit(MachineKey::from_book("fixture"), NodeId::random())
            .unwrap(),
        worktree: WorktreeId::random(),
    };
    let mut model = Model::default();
    model.select(Some(address.clone()), Support::ObserveAndCancel);
    let key = model.begin_read().unwrap();
    assert!(model.accept_read(
        &address,
        key,
        Ok(SetupProgressReply {
            progress: Some(SetupProgress {
                worktree: address.worktree,
                cursor: SetupCursor {
                    run: SetupRunId::random(),
                    revision: SetupRevision::ZERO
                },
                state: SetupRunState::Running,
                started_at: chrono::DateTime::UNIX_EPOCH,
                commands: vec![SetupCommandProgress {
                    index: SetupCommandIndex::try_from(1).unwrap(),
                    command: "printf fixture".parse().unwrap(),
                    state: result.map_or(SetupCommandState::Running, |result| {
                        SetupCommandState::Finished {
                            elapsed: DurationMs::from_duration(std::time::Duration::from_secs(61)),
                            result,
                        }
                    }),
                    output: TerminalBytes::new(b"progress\rcomplete\n".to_vec().into()),
                    truncated: true,
                }],
            }),
        })
    ));
    model
}
#[gpui::test]
fn shared_kit_paints_structured_command_results_in_both_languages_and_schemes(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
        model: fixture(None),
        scheme: Scheme::Dark,
    });
    for language in ["en", "ko"] {
        for scheme in [Scheme::Dark, Scheme::Light] {
            for result in [
                None,
                Some(SetupCommandResult::Exited { code: Some(0) }),
                Some(SetupCommandResult::Exited { code: Some(7) }),
                Some(SetupCommandResult::Exited { code: None }),
                Some(SetupCommandResult::SpawnFailed),
                Some(SetupCommandResult::TimedOut),
                Some(SetupCommandResult::Cancelled),
                Some(SetupCommandResult::IoFailed),
                Some(SetupCommandResult::Unknown),
            ] {
                probe.update_in(cx, |probe, _window, cx| {
                    probe.model = fixture(result);
                    probe.scheme = scheme;
                    probe.shell.update(cx, |shell, _cx| {
                        shell.words = Translator::get(Locale::from_tag(language).unwrap()).unwrap();
                    });
                    cx.notify();
                });
                cx.run_until_parked();
                assert!(cx.debug_bounds("setup-command").is_some());
            }
        }
    }
}
#[gpui::test]
fn display_values_alone_cannot_dispatch_cancel_without_an_admitted_controller(
    cx: &mut TestAppContext,
) {
    language_tests::init(cx);
    let (probe, cx) = cx.add_window_view(|window, cx| Probe {
        shell: cx.new(|cx| Shell::without_io(window, cx)),
        model: fixture(None),
        scheme: Scheme::Dark,
    });
    cx.run_until_parked();
    let cancel = cx.debug_bounds("setup-observation-cancel").unwrap();
    assert!(probe.read_with(cx, |probe, _cx| probe.model.can_cancel()));
    cx.simulate_click(cancel.center(), Modifiers::default());
    cx.run_until_parked();
    assert!(probe.read_with(cx, |probe, _cx| probe.model.can_cancel()));
    assert_eq!(
        probe.read_with(cx, |probe, _cx| probe.model.cancellation),
        None
    );
}
