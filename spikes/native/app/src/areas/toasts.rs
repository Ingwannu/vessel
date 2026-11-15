//! One current toast above the window, with a keyed hover-paused countdown.

use std::{rc::Rc, time::Instant};

use vessel_app_kit::ui::{AnyElement, Context, ElementId, Window};
use vessel_app_kit::{ToastContent, ToastHandlers, ToastLook, Tokens, chrome, toast_with, toasts};
use vessel_app_view::toasts::ToastTone;

use crate::shell::Shell;

/// The room between the title bar and the first toast.
const GAP: f32 = 8.0;

impl Shell {
    /// Show feedback with the reference default or error-helper lifetime.
    pub(crate) fn toast(
        &mut self,
        text: impl Into<String>,
        tone: ToastTone,
        cx: &mut Context<'_, Self>,
    ) {
        let (id, _life) = self.areas.toasts.push(text, tone);
        if let Some(wake) = self.areas.toasts.start(id, Instant::now()) {
            Self::schedule_toast(wake, cx);
        }
        cx.notify();
    }

    /// Guidance keeps the caller's explicit 3200ms reading time; generic Info
    /// uses the ordinary 2200ms default as every other show variant does.
    pub(crate) fn guidance_toast(&mut self, text: impl Into<String>, cx: &mut Context<'_, Self>) {
        let (id, _life) = self.areas.toasts.push_with(
            text,
            ToastTone::Info,
            vessel_app_view::toasts::Lifetime::Timed(vessel_app_view::toasts::INFO_LIFE),
        );
        if let Some(wake) = self.areas.toasts.start(id, Instant::now()) {
            Self::schedule_toast(wake, cx);
        }
        cx.notify();
    }

    fn schedule_toast(wake: vessel_app_view::toasts::ToastWake, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wake.after()).await;
            let _gone = this.update(cx, |shell, cx| {
                if let Some(next) = shell.areas.toasts.wake(wake, Instant::now()) {
                    Self::schedule_toast(next, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn hover_toast(
        &mut self,
        id: u64,
        event: vessel_app_kit::ToastHover,
        cx: &mut Context<'_, Self>,
    ) {
        let hover = match event {
            vessel_app_kit::ToastHover::Enter => vessel_app_view::toasts::ToastHover::Enter,
            vessel_app_kit::ToastHover::Leave => vessel_app_view::toasts::ToastHover::Leave,
        };
        if let Some(wake) = self.areas.toasts.hover(id, hover, Instant::now()) {
            Self::schedule_toast(wake, cx);
        }
    }

    /// Take toast `id` away.
    pub(crate) fn dismiss_toast(&mut self, id: u64, cx: &mut Context<'_, Self>) {
        self.areas.toasts.dismiss(id);
        cx.notify();
    }
}

/// The toasts, under the title bar.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let cards = shell
        .areas
        .toasts
        .list()
        .iter()
        .map(|said| {
            let id = said.id;
            let look = match said.tone {
                ToastTone::Default => ToastLook::Plain,
                ToastTone::Ok => ToastLook::Success,
                ToastTone::Warning => ToastLook::Warning,
                ToastTone::Info => ToastLook::Info,
                ToastTone::Error => ToastLook::Error,
            };
            let hover = Some({
                let handler = cx.listener(
                    move |shell: &mut Shell, event: &vessel_app_kit::ToastHover, _window, cx| {
                        shell.hover_toast(id, *event, cx);
                    },
                );
                let callback: vessel_app_kit::ToastHovered =
                    Rc::new(move |event, window, cx| handler(&event, window, cx));
                callback
            });
            toast_with(
                ToastContent {
                    id: ElementId::NamedInteger("toast".into(), id),
                    text: said.text.clone().into(),
                    look,
                },
                ToastHandlers {
                    dismiss: Rc::new(
                        cx.listener(move |shell, _event, _window, cx| shell.dismiss_toast(id, cx)),
                    ),
                    hover,
                },
                tokens,
            )
        })
        .collect();
    toasts(cards, chrome::HEIGHT.mul_add(1.0, GAP), window)
}
