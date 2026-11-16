//! Unregistered Host Voice surface over canonical values and caller-owned actions.
//!
//! Settings supplies rows/switches/alerts. This area owns composition only;
//! callbacks carry the displayed host/ticket and must pass Page's dispatch check.
//! No Shell route, node field, provider or key-entry control is installed here.

use vessel_app_kit::theme::{icon, spacing};
use vessel_app_kit::ui::{
    App, ClickEvent, Div, InteractiveElement as _, IntoElement as _, ParentElement as _,
    SharedString, StatefulInteractiveElement as _, Styled as _, Window, div, px,
};
use vessel_app_kit::{ControlSizing, Tokens, Type, TypeExt as _, assets::Glyph, glyph, settings};
use vessel_app_view::{
    localization::Translator,
    voice::settings::{Edit, Feature, Page, Problem, Site, Snapshot, Status, Toggle},
};
use vessel_proto::dictation::{DictationProvider, DictationSwitch, KeyPresence};

pub(super) struct Actions<T, E> {
    pub(super) toggle: T,
    pub(super) edit: E,
}

pub(super) fn render<T, E>(
    page: &Page,
    words: &'static Translator,
    tokens: &Tokens,
    sizing: ControlSizing,
    actions: Actions<T, E>,
) -> Div
where
    T: Fn(Toggle, &ClickEvent, &mut Window, &mut App) + Clone + 'static,
    E: Fn(Edit, &ClickEvent, &mut Window, &mut App) + Clone + 'static,
{
    let content = match page.status() {
        Status::Loading => return loading(words, tokens),
        Status::Failed(problem) => return notice(problem, words, tokens),
        Status::Ready(snapshot, _, failure) => (snapshot, failure),
    };
    let (snapshot, failure) = content;
    let dictation = card(
        page,
        snapshot,
        Feature::Dictation,
        words,
        tokens,
        sizing,
        Actions {
            toggle: actions.toggle.clone(),
            edit: actions.edit.clone(),
        },
    );
    let conversation = card(
        page,
        snapshot,
        Feature::Conversation,
        words,
        tokens,
        sizing,
        actions,
    );
    let mut body = div()
        .flex()
        .flex_col()
        .min_w_0()
        .gap(px(spacing::S6))
        .debug_selector(|| "host-voice-settings".to_owned())
        .child(settings::section(
            SharedString::new_static(words.voice().settings_dictation_title()),
            dictation,
            tokens,
        ))
        .child(settings::section(
            SharedString::new_static(words.voice().settings_conversation_title()),
            conversation,
            tokens,
        ));
    if failure.is_some() {
        body = body.child(message(words.voice().settings_save_error(), tokens));
    }
    body
}

fn card<T, E>(
    page: &Page,
    snapshot: &Snapshot,
    feature: Feature,
    words: &'static Translator,
    tokens: &Tokens,
    sizing: ControlSizing,
    actions: Actions<T, E>,
) -> Div
where
    T: Fn(Toggle, &ClickEvent, &mut Window, &mut App) + 'static,
    E: Fn(Edit, &ClickEvent, &mut Window, &mut App) + Clone + 'static,
{
    let (value, sites): (_, &[Site]) = match feature {
        Feature::Dictation => (snapshot.dictation.switch, &[Site::Dictation]),
        Feature::Conversation => (
            snapshot.conversation.switch,
            &[Site::Recognition, Site::Synthesis],
        ),
    };
    let mut card = settings::card(tokens).child(switch_row(
        page,
        feature,
        value,
        words,
        tokens,
        sizing,
        actions.toggle,
    ));
    for site in sites {
        card = card.child(site_row(
            page,
            snapshot,
            *site,
            words,
            tokens,
            sizing,
            actions.edit.clone(),
        ));
    }
    card
}

fn switch_words(
    feature: Feature,
    words: &'static Translator,
) -> (&'static str, &'static str, &'static str) {
    match feature {
        Feature::Dictation => (
            "voice-settings-dictation",
            words.voice().settings_dictation_switch(),
            words.voice().settings_dictation_hint(),
        ),
        Feature::Conversation => (
            "voice-settings-conversation",
            words.voice().settings_conversation_switch(),
            words.voice().settings_conversation_hint(),
        ),
    }
}

fn switch_row<T>(
    page: &Page,
    feature: Feature,
    value: DictationSwitch,
    words: &'static Translator,
    tokens: &Tokens,
    sizing: ControlSizing,
    action: T,
) -> Div
where
    T: Fn(Toggle, &ClickEvent, &mut Window, &mut App) + 'static,
{
    let (id, title, hint) = switch_words(feature, words);
    if value == DictationSwitch::Unknown {
        let unknown = div()
            .id(id)
            .debug_selector(move || id.to_owned())
            .type_(Type::Meta)
            .text_color(tokens.neutral.foreground_muted)
            .child(SharedString::new_static(
                words.voice().settings_unknown_switch(),
            ));
        return settings::row_at(
            SharedString::new_static(title),
            Some(SharedString::new_static(hint)),
            None,
            Some(unknown.into_any_element()),
            true,
            sizing == ControlSizing::Touch,
            tokens,
        );
    }
    let request = page.toggle_action(feature);
    let mut control = settings::switch(
        id,
        value == DictationSwitch::On,
        request.is_some(),
        sizing == ControlSizing::Touch,
        0,
        tokens,
    );
    control = control
        .role(gpui::Role::Switch)
        .aria_label(title)
        .aria_toggled(if value == DictationSwitch::On {
            gpui::Toggled::True
        } else {
            gpui::Toggled::False
        })
        .debug_selector(move || id.to_owned());
    if let Some(request) = request {
        control = control.on_click(move |event, window, cx| action(request, event, window, cx));
    }
    settings::row_at(
        SharedString::new_static(title),
        Some(SharedString::new_static(hint)),
        None,
        Some(control.into_any_element()),
        true,
        sizing == ControlSizing::Touch,
        tokens,
    )
}

fn site_words(site: Site, words: &'static Translator) -> (&'static str, &'static str) {
    match site {
        Site::Dictation => (
            "voice-settings-dictation-site",
            words.voice().settings_dictation_site(),
        ),
        Site::Recognition => (
            "voice-settings-recognition-site",
            words.voice().settings_recognition_site(),
        ),
        Site::Synthesis => (
            "voice-settings-synthesis-site",
            words.voice().settings_synthesis_site(),
        ),
    }
}

fn site_row<E>(
    page: &Page,
    snapshot: &Snapshot,
    site: Site,
    words: &'static Translator,
    tokens: &Tokens,
    sizing: ControlSizing,
    action: E,
) -> Div
where
    E: Fn(Edit, &ClickEvent, &mut Window, &mut App) + 'static,
{
    let (id, title) = site_words(site, words);
    let request = page.edit_action(site);
    let summary = summary(snapshot, site, words);
    let label = words
        .voice()
        .settings_site_label(title, &summary)
        .unwrap_or_else(|_| title.to_owned());
    let mut trailing = div()
        .flex()
        .items_center()
        .min_w_0()
        .gap(px(spacing::S2))
        .child(
            div()
                .min_w_0()
                .type_(Type::Meta)
                .text_color(tokens.neutral.foreground_muted)
                .truncate()
                .child(summary),
        );
    if request.is_some() {
        trailing = trailing.child(
            glyph(
                Glyph::ChevronRight,
                icon::SM,
                tokens.neutral.foreground_muted,
            )
            .mr(px(-spacing::S1)),
        );
    }
    let title = div()
        .flex_1()
        .min_w_0()
        .type_(Type::Body)
        .text_color(tokens.neutral.foreground)
        .child(SharedString::new_static(title));
    let row = settings::row_with_content(
        title.into_any_element(),
        Some(trailing.into_any_element()),
        false,
        sizing == ControlSizing::Touch,
        tokens,
    );
    let mut row = row
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .debug_selector(move || id.to_owned());
    if let Some(request) = request {
        let hover = tokens.neutral.surface2;
        let pressed = tokens.neutral.surface3;
        row = row
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .active(move |style| style.bg(pressed))
            .on_click(move |event, window, cx| action(request, event, window, cx));
    }
    div().child(row)
}

fn site_values(snapshot: &Snapshot, site: Site) -> (DictationProvider, KeyPresence, &str, &str) {
    match site {
        Site::Dictation => {
            let s = &snapshot.dictation;
            (
                s.provider,
                s.key,
                s.local_model.as_str(),
                s.endpoint_model.as_str(),
            )
        }
        Site::Recognition => {
            let s = &snapshot.conversation.stt;
            (
                s.provider,
                s.key,
                s.local_model.as_str(),
                s.endpoint_model.as_str(),
            )
        }
        Site::Synthesis => {
            let s = &snapshot.conversation.tts;
            (
                s.provider,
                s.key,
                s.local_model.as_str(),
                s.endpoint_model.as_str(),
            )
        }
    }
}

fn summary(snapshot: &Snapshot, site: Site, words: &Translator) -> String {
    let (provider, key, local, endpoint) = site_values(snapshot, site);
    match provider {
        DictationProvider::Local => words
            .voice()
            .settings_local_model(local)
            .unwrap_or_else(|_| words.voice().settings_local().to_owned()),
        DictationProvider::Endpoint if key == KeyPresence::Absent => {
            words.voice().settings_key_missing().to_owned()
        }
        DictationProvider::Endpoint => words
            .voice()
            .settings_endpoint_model(endpoint)
            .unwrap_or_else(|_| words.voice().settings_endpoint().to_owned()),
        DictationProvider::Unknown => words.voice().settings_unknown().to_owned(),
    }
}

fn notice(problem: Problem, words: &Translator, tokens: &Tokens) -> Div {
    use vessel_app_kit::alert::{self, Content, Size, Variant};
    let text = match problem {
        Problem::Offline => words.voice().settings_offline(),
        Problem::Unsupported => words.voice().settings_unsupported(),
        Problem::Refused(_) => words.voice().settings_read_error(),
    };
    alert::alert(
        Content {
            title: Some(text.to_owned().into()),
            description: None,
            action: None,
        },
        Variant::Warning,
        Size::Medium,
        tokens,
    )
}
fn message(text: &str, tokens: &Tokens) -> Div {
    div()
        .type_(Type::Body)
        .text_color(tokens.danger)
        .child(text.to_owned())
}
// React Native Web's ActivityIndicator large preset is36px; derive it from
// existing spacing tokens rather than introduce another control renderer.
const LOADING_SPINNER_SIZE: f32 = spacing::S6 + spacing::S3;
fn loading(words: &Translator, tokens: &Tokens) -> Div {
    div()
        .id("voice-settings-loading-panel")
        .role(gpui::Role::ProgressIndicator)
        .aria_label(words.voice().settings_loading().to_owned())
        .py(px(spacing::S8))
        .flex()
        .items_center()
        .justify_center()
        .child(vessel_app_kit::slider::spinner(
            "voice-settings-loading",
            LOADING_SPINNER_SIZE,
            tokens.neutral.foreground_muted,
        ))
}

#[cfg(test)]
mod tests;
