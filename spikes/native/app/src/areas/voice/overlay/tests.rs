//! Production composition with typed fake state: no shell, node or audio device.
use super::*;
use gpui::{Context, Modifiers, Render, TestAppContext, VisualTestContext, size};
use vessel_app_kit::Scheme;
use vessel_app_view::{
    localization::Locale,
    voice::{Availability, Level, Target},
};

#[derive(Debug, Clone, Copy)]
enum Case {
    Open,
    Muted,
    Starting,
    StoppingMuted,
}
fn state(case: Case) -> State {
    let mut state = State::default();
    let generation = state
        .start(
            Target {
                node: vessel_proto::NodeId::random(),
                session: vessel_proto::SessionId::random(),
            },
            Availability::Ready,
        )
        .unwrap();
    if matches!(case, Case::Starting) {
        return state;
    }
    let _ready = state.node_ready(generation).unwrap();
    let _capturing = state.capture_ready(generation).unwrap();
    if matches!(case, Case::Muted | Case::StoppingMuted) {
        let _mute = state.toggle_mute().unwrap();
    }
    if matches!(case, Case::StoppingMuted) {
        let _stopping = state.stop().unwrap();
    }
    state
}
fn telemetry() -> Telemetry {
    Telemetry {
        level: Level::new(0.4).unwrap(),
        activity: SpeechActivity::Quiet,
        mute: Mute::Open,
    }
}
struct Probe {
    state: State,
    scheme: Scheme,
    words: &'static Translator,
    clicks: [usize; 2],
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let model =
            Model::new(&self.state, telemetry(), telemetry(), "voice-meter".into()).unwrap();
        let actions = Actions {
            mute: cx.listener(|this, _event, _window, cx| {
                if this.state.toggle_mute().is_ok() {
                    this.clicks[0] = this.clicks[0].saturating_add(1);
                    cx.notify();
                }
            }),
            stop: cx.listener(|this, _event, _window, cx| {
                if this.state.stop().is_ok() {
                    this.clicks[1] = this.clicks[1].saturating_add(1);
                    cx.notify();
                }
            }),
        };
        div().size_full().children(super::render(
            &model,
            self.words,
            &Tokens::of(self.scheme),
            actions,
        ))
    }
}
fn words(tag: &str) -> &'static Translator {
    Translator::get(Locale::from_tag(tag).unwrap()).unwrap()
}

#[gpui::test]
fn actual_overlay_keeps_footer_touch_targets_and_meter_space_at_phone_and_desktop_widths(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| cx.set_reduce_motion(true));
    for width in [390.0, 800.0, 1280.0] {
        for scheme in [Scheme::Light, Scheme::Dark] {
            for tag in ["en", "ko"] {
                for case in [Case::Open, Case::Muted, Case::Starting, Case::StoppingMuted] {
                    let window = cx.open_window(size(px(width), px(100.0)), move |_, _| Probe {
                        state: state(case),
                        scheme,
                        words: words(tag),
                        clicks: [0; 2],
                    });
                    let mut visual = VisualTestContext::from_window(window.into(), cx);
                    visual.run_until_parked();
                    let frame = visual.debug_bounds("voice-overlay").unwrap();
                    let mute = visual.debug_bounds("voice-mute").unwrap();
                    let stop = visual.debug_bounds("voice-stop").unwrap();
                    let meter = visual.debug_bounds("voice-meter-slot").unwrap();
                    assert_eq!(frame.size, size(px(width), px(75.0)));
                    assert_eq!(mute.size, size(px(44.0), px(44.0)));
                    assert_eq!(stop.size, mute.size);
                    assert_eq!(mute.center().y, stop.center().y);
                    assert_eq!(stop.left(), mute.right() + px(8.0));
                    assert!(frame.contains(mute.origin) && frame.contains(stop.bottom_right()));
                    assert!(meter.right() <= mute.left());
                    assert!(meter.size.width > px(0.0));
                }
            }
        }
    }
}

#[gpui::test]
fn real_kit_handlers_toggle_mute_and_stop_once_while_switching_refuses_both(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| cx.set_reduce_motion(true));
    for case in [Case::Open, Case::Starting, Case::StoppingMuted] {
        let (view, visual) = cx.add_window_view(move |_, _| Probe {
            state: state(case),
            scheme: Scheme::Dark,
            words: words("ko"),
            clicks: [0; 2],
        });
        let mute = visual.debug_bounds("voice-mute").unwrap();
        let stop = visual.debug_bounds("voice-stop").unwrap();
        visual.simulate_click(mute.center(), Modifiers::none());
        visual.run_until_parked();
        visual.simulate_click(stop.center(), Modifiers::none());
        visual.run_until_parked();
        visual.simulate_click(mute.center(), Modifiers::none());
        visual.simulate_click(stop.center(), Modifiers::none());
        visual.run_until_parked();
        view.read_with(visual, |probe, _| {
            assert_eq!(
                probe.clicks,
                if matches!(case, Case::Open) {
                    [1, 1]
                } else {
                    [0, 0]
                }
            );
        });
    }
}

#[test]
fn disabled_voice_has_no_overlay_and_state_mute_overrides_stale_telemetry() {
    let before = telemetry();
    let muted = Model::new(&state(Case::Muted), before, before, "meter".into()).unwrap();
    assert_eq!(muted.meter.after, meter::State::Muted);
    let model = Model::new(&State::default(), before, before, "meter".into()).unwrap();
    let actions = Actions {
        mute: |_event: &ClickEvent, _window: &mut Window, _cx: &mut App| {},
        stop: |_event: &ClickEvent, _window: &mut Window, _cx: &mut App| {},
    };
    assert!(super::render(&model, words("en"), &Tokens::of(Scheme::Dark), actions).is_none());
    assert_eq!(words("en").voice().mute(), "Mute realtime voice");
    assert_eq!(words("ko").voice().unmute(), "실시간 음성 음소거 해제");
}
