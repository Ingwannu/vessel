//! Notification settings use the existing persisted preferences and kit rows.
//! The sound and desktop switches independently update the same alert mode.
use crate::shell::Shell;
use vessel_app_kit::ui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, StatefulInteractiveElement as _,
    Window,
};
use vessel_app_kit::{Tokens, outline_button, settings};
use vessel_app_view::inbox::delivery::{Channel, Delivery, Toggle};
use vessel_app_view::prefs::PreferenceChange;

pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    compact: bool,
    _window: &mut Window,
    cx: &mut Context<'_, Shell>,
) -> AnyElement {
    let words = shell.words.inbox();
    let delivery = Delivery::of(shell.prefs.settings.notifications.alerts);
    let desktop = settings::switch(
        "notifications-desktop",
        delivery.desktop == Channel::Enabled,
        true,
        compact,
        0,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        let next = Delivery::of(shell.prefs.settings.notifications.alerts).toggled(Toggle::Desktop);
        shell.prefer(&PreferenceChange::Alerts(next), window, cx);
    }));
    let sound = settings::switch(
        "notifications-sound",
        delivery.sound == Channel::Enabled,
        true,
        compact,
        0,
        tokens,
    )
    .on_click(cx.listener(move |shell, _event, window, cx| {
        let next = Delivery::of(shell.prefs.settings.notifications.alerts).toggled(Toggle::Sound);
        shell.prefer(&PreferenceChange::Alerts(next), window, cx);
    }));
    let title = settings::switch(
        "notifications-title",
        shell.prefs.settings.notifications.title,
        true,
        compact,
        0,
        tokens,
    )
    .on_click(cx.listener(|shell, _event, window, cx| {
        shell.prefer(
            &PreferenceChange::TitleCount(!shell.prefs.settings.notifications.title),
            window,
            cx,
        );
    }));
    let test = outline_button("notifications-test", words.test(), true, tokens).on_click(
        cx.listener(|shell, _event, window, cx| {
            shell.areas.inbox.test_notice = vessel_app_view::inbox::settings::TestNotice::Requested;
            cx.notify();
            cx.show_system_notification(vessel_app_kit::ui::SystemNotification {
                tag: "vessel-notification-test".into(),
                title: shell.t.app.into(),
                body: shell.words.inbox().test().to_owned().into(),
                actions: Vec::new(),
            });
            if Delivery::of(shell.prefs.settings.notifications.alerts).sound == Channel::Enabled {
                window.play_system_bell();
            }
        }),
    );
    let card = settings::card(tokens)
        .child(settings::row_at(
            words.permission(),
            Some(words.os_managed().into()),
            None,
            None,
            true,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.desktop_alerts(),
            Some(words.desktop_hint().into()),
            None,
            Some(desktop.into_any_element()),
            false,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.play_sound(),
            Some(words.sound_hint().into()),
            None,
            Some(sound.into_any_element()),
            false,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.title_count(),
            None,
            None,
            Some(title.into_any_element()),
            false,
            compact,
            tokens,
        ))
        .child(settings::row_at(
            words.test(),
            Some(words.platform_hint().into()),
            None,
            Some(test.into_any_element()),
            false,
            compact,
            tokens,
        ));
    let card = match shell.areas.inbox.test_notice {
        vessel_app_view::inbox::settings::TestNotice::Idle => card,
        vessel_app_view::inbox::settings::TestNotice::Requested => card.child(settings::row_at(
            words.test_requested(),
            Some(words.test_unconfirmed().into()),
            None,
            None,
            false,
            compact,
            tokens,
        )),
    };
    settings::section(words.notifications(), card, tokens).into_any_element()
}
