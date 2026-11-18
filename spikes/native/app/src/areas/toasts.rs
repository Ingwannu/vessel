//! The toast over the window (the page's `ui/toasts.tsx`, drawn as Paseo's
//! toast host): what an action came to, when the screen it was asked from
//! has nowhere else to say it. One at a time: a new toast replaces the one
//! up and runs its own life (`vessel_app_view::toasts`), held while a
//! pointer rests on it.

use std::{rc::Rc, time::Instant};

use vessel_app_kit::ui::{AnyElement, Context, ElementId, Window};
use vessel_app_kit::{
    ToastContent, ToastHover, ToastHovered, ToastLook, Tokens, chrome, toast_layer, toast_with,
};
use vessel_app_view::toasts::{ToastTone, ToastWake};

use crate::shell::Shell;

/// The room between the title bar and the toast.
const GAP: f32 = 8.0;

impl Shell {
    /// Say `text` in a toast, in place of the one up.
    pub(crate) fn toast(
        &mut self,
        text: impl Into<String>,
        tone: ToastTone,
        cx: &mut Context<'_, Self>,
    ) {
        let wake = self.areas.toasts.push(text, tone, Instant::now());
        Self::schedule_toast(wake, cx);
        cx.notify();
    }

    /// Paseo's `toast.copied(label)`: "Copied {label}" (`layout-copied`, its
    /// `common.states.copiedLabel`) as a copied toast, for every copy that
    /// says so (TRACKER 804).
    pub(crate) fn toast_copied(&mut self, label: &str, cx: &mut Context<'_, Self>) {
        let words = self.words.layout();
        let text = words.copied(label).unwrap_or_else(|_| label.to_owned());
        self.toast(text, ToastTone::Copied, cx);
    }

    fn schedule_toast(wake: ToastWake, cx: &mut Context<'_, Self>) {
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

    fn hover_toast(&mut self, id: u64, event: ToastHover, cx: &mut Context<'_, Self>) {
        let hover = match event {
            ToastHover::Enter => vessel_app_view::toasts::ToastHover::Enter,
            ToastHover::Leave => vessel_app_view::toasts::ToastHover::Leave,
        };
        if let Some(wake) = self.areas.toasts.hover(id, hover, Instant::now()) {
            Self::schedule_toast(wake, cx);
        }
    }
}

/// The toast, under the title bar.
pub(crate) fn render(
    shell: &Shell,
    tokens: &Tokens,
    window: &Window,
    cx: &mut Context<'_, Shell>,
) -> Option<AnyElement> {
    let said = shell.areas.toasts.shown()?;
    let id = said.id;
    let look = match said.tone {
        ToastTone::Ok => ToastLook::Success,
        ToastTone::Copied => ToastLook::Copied,
        ToastTone::Info => ToastLook::Info,
        ToastTone::Error => ToastLook::Error,
    };
    let handler = cx.listener(move |shell: &mut Shell, event: &ToastHover, _window, cx| {
        shell.hover_toast(id, *event, cx);
    });
    let hover: ToastHovered = Rc::new(move |event, window, cx| handler(&event, window, cx));
    // One element for whichever toast is up (Paseo's single surface): a
    // replacement changes its words without a second arrival, and a pointer
    // resting on it stays on it.
    let card = toast_with(
        ToastContent {
            id: ElementId::Name("toast".into()),
            text: said.text.clone().into(),
            look,
        },
        Some(hover),
        tokens,
        window,
    );
    Some(toast_layer(card, chrome::HEIGHT.mul_add(1.0, GAP), window))
}
