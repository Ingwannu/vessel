//! Headless composition/actions only: no Shell route, RPC, provider or secret field.
use super::*;
use gpui::{Context, IntoElement, Modifiers, Render, TestAppContext, VisualTestContext, size};
use vessel_app_kit::Scheme;
use vessel_app_view::{
    devices::Tickets,
    localization::Locale,
    voice::{Applied, settings::Write},
};
use vessel_proto::{NodeId, dictation::DictationSettings, voice::VoiceSettings};

struct Probe {
    page: Page,
    tickets: Tickets,
    words: &'static Translator,
    scheme: Scheme,
    sizing: ControlSizing,
    writes: Vec<Write>,
    editors: Vec<Site>,
}
impl Probe {
    fn new(tag: &str, scheme: Scheme, sizing: ControlSizing) -> Self {
        let mut tickets = Tickets::default();
        let ticket = tickets.next_ticket().unwrap();
        let node = NodeId::random();
        let mut page = Page::reading(node, ticket);
        assert_eq!(
            page.answered(node, ticket, Ok(snapshot())),
            Applied::Current
        );
        Self {
            page,
            tickets,
            words: Translator::get(Locale::from_tag(tag).unwrap()).unwrap(),
            scheme,
            sizing,
            writes: Vec::new(),
            editors: Vec::new(),
        }
    }
}
fn snapshot() -> Snapshot {
    Snapshot {
        dictation: DictationSettings::default(),
        conversation: VoiceSettings::default(),
    }
}
impl Render for Probe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let toggle_owner = cx.entity().downgrade();
        let edit_owner = cx.entity().downgrade();
        let actions = Actions {
            toggle: move |action, _event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                let _retired = toggle_owner.update(cx, |this, cx| {
                    let ticket = this.tickets.next_ticket().unwrap();
                    if let Some(write) = this.page.toggle(action, ticket) {
                        this.writes.push(write);
                        cx.notify();
                    }
                });
            },
            edit: move |action, _event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                let _retired = edit_owner.update(cx, |this, _| {
                    if let Some(site) = this.page.edit(action) {
                        this.editors.push(site);
                    }
                });
            },
        };
        div().size_full().flex().flex_col().child(render(
            &self.page,
            self.words,
            &Tokens::of(self.scheme),
            self.sizing,
            actions,
        ))
    }
}

#[gpui::test]
fn host_cards_keep_all_three_site_rows_within_phone_and_desktop_bounds(cx: &mut TestAppContext) {
    cx.update(|cx| cx.set_reduce_motion(true));
    for (width, sizing) in [
        (390.0, ControlSizing::Touch),
        (800.0, ControlSizing::Pointer),
        (1280.0, ControlSizing::Pointer),
    ] {
        for scheme in [Scheme::Light, Scheme::Dark] {
            for tag in ["en", "ko"] {
                let window = cx.open_window(size(px(width), px(844.0)), move |_, _| {
                    Probe::new(tag, scheme, sizing)
                });
                let mut visual = VisualTestContext::from_window(window.into(), cx);
                visual.run_until_parked();
                let body = visual.debug_bounds("host-voice-settings").unwrap();
                for id in [
                    "voice-settings-dictation-site",
                    "voice-settings-recognition-site",
                    "voice-settings-synthesis-site",
                    "voice-settings-dictation",
                    "voice-settings-conversation",
                ] {
                    let bounds = visual.debug_bounds(id).unwrap();
                    assert!(body.contains(&bounds.origin));
                    assert!(body.contains(&bounds.bottom_right()));
                    assert!(bounds.right() <= px(width));
                }
            }
        }
    }
}

#[gpui::test]
fn actual_switch_handler_is_single_pending_and_site_click_waits_for_confirmation(
    cx: &mut TestAppContext,
) {
    cx.update(|cx| cx.set_reduce_motion(true));
    let (view, visual) =
        cx.add_window_view(|_, _| Probe::new("ko", Scheme::Dark, ControlSizing::Touch));
    let first = visual.debug_bounds("voice-settings-dictation").unwrap();
    visual.simulate_click(first.center(), Modifiers::none());
    visual.run_until_parked();
    let second = visual.debug_bounds("voice-settings-conversation").unwrap();
    visual.simulate_click(second.center(), Modifiers::none());
    let site = visual
        .debug_bounds("voice-settings-synthesis-site")
        .unwrap();
    visual.simulate_click(site.center(), Modifiers::none());
    visual.run_until_parked();
    view.update(visual, |probe, cx| {
        assert_eq!(probe.writes.len(), 1);
        assert!(probe.editors.is_empty());
        let request = probe.writes.first().unwrap();
        let mut values = snapshot();
        values.dictation.switch = DictationSwitch::Off;
        assert_eq!(
            probe
                .page
                .answered(request.node, request.ticket, Ok(values)),
            Applied::Current
        );
        cx.notify();
    });
    visual.run_until_parked();
    let site = visual
        .debug_bounds("voice-settings-synthesis-site")
        .unwrap();
    visual.simulate_click(site.center(), Modifiers::none());
    view.read_with(visual, |probe, _| {
        assert_eq!(probe.editors, [Site::Synthesis]);
    });
}

#[test]
fn summaries_and_unavailability_are_complete_localized_messages() {
    for (tag, offline) in [
        ("en", "Connect to this host to configure speech."),
        ("ko", "음성을 설정하려면 이 호스트에 연결하세요."),
    ] {
        let words = Translator::get(Locale::from_tag(tag).unwrap()).unwrap();
        assert_eq!(words.voice().settings_offline(), offline);
        let mut values = snapshot();
        assert!(
            summary(&values, Site::Dictation, words)
                .contains(values.dictation.local_model.as_str())
        );
        values.conversation.tts.provider = DictationProvider::Endpoint;
        assert_eq!(
            summary(&values, Site::Synthesis, words),
            words.voice().settings_key_missing()
        );
        values.conversation.tts.provider = DictationProvider::Unknown;
        assert_eq!(
            summary(&values, Site::Synthesis, words),
            words.voice().settings_unknown()
        );
    }
}
