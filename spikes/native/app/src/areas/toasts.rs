//! The toasts over the window (the page's `ui/toasts.tsx`, drawn as
//! Paseo's toast host): what an action came to, when the screen it was
//! asked from has nowhere else to say it. Each stays its life
//! (`vessel_app_view::toasts`), an error until dismissed.

use std::{rc::Rc, time::Instant};

use vessel_app_kit::ui::{AnyElement, Context, ElementId, Window};
use vessel_app_kit::{ToastContent, ToastHandlers, ToastLook, Tokens, chrome, toast_with, toasts};
use vessel_app_view::toasts::ToastTone;

use crate::shell::Shell;

/// The room between the title bar and the first toast.
const GAP: f32 = 8.0;

impl Shell {
    /// Say `text` in a toast; one that is not an error leaves by itself.
    pub(crate) fn toast(
        &mut self,
        text: impl Into<String>,
        tone: ToastTone,
        cx: &mut Context<'_, Self>,
    ) {
        let (id, life) = self.areas.toasts.push(text, tone);
        if tone == ToastTone::Info {
            if let Some(wake) = self.areas.toasts.start_info(id, Instant::now()) {
                Self::schedule_info_toast(wake, cx);
            }
        } else if let Some(life) = life {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(life).await;
                let _gone = this.update(cx, |shell, cx| shell.dismiss_toast(id, cx));
            })
            .detach();
        }
        cx.notify();
    }

    fn schedule_info_toast(wake: vessel_app_view::toasts::InfoWake, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wake.after()).await;
            let _gone = this.update(cx, |shell, cx| {
                if let Some(next) = shell.areas.toasts.wake_info(wake, Instant::now()) {
                    Self::schedule_info_toast(next, cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn hover_info_toast(
        &mut self,
        id: u64,
        event: vessel_app_kit::ToastHover,
        cx: &mut Context<'_, Self>,
    ) {
        let hover = match event {
            vessel_app_kit::ToastHover::Enter => vessel_app_view::toasts::ToastHover::Enter,
            vessel_app_kit::ToastHover::Leave => vessel_app_view::toasts::ToastHover::Leave,
        };
        if let Some(wake) = self.areas.toasts.hover_info(id, hover, Instant::now()) {
            Self::schedule_info_toast(wake, cx);
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
                ToastTone::Ok => ToastLook::Plain,
                ToastTone::Info => ToastLook::Info,
                ToastTone::Error => ToastLook::Error,
            };
            let hover = (said.tone == ToastTone::Info).then(|| {
                let handler = cx.listener(
                    move |shell: &mut Shell, event: &vessel_app_kit::ToastHover, _window, cx| {
                        shell.hover_info_toast(id, *event, cx);
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
