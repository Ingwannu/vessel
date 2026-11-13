//! The toasts over the window (the page's `ui/toasts.tsx`, drawn as
//! Paseo's toast host): what an action came to, when the screen it was
//! asked from has nowhere else to say it. Each stays its life
//! (`vessel_app_view::toasts`), an error until dismissed.

use vessel_app_kit::ui::{AnyElement, Context, ElementId, Window};
use vessel_app_kit::{ToastLook, Tokens, chrome, toast, toasts};
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
        if let Some(life) = life {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(life).await;
                let _gone = this.update(cx, |shell, cx| shell.dismiss_toast(id, cx));
            })
            .detach();
        }
        cx.notify();
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
                ToastTone::Error => ToastLook::Error,
            };
            toast(
                ElementId::NamedInteger("toast".into(), id),
                said.text.clone(),
                look,
                cx.listener(move |shell, _event, _window, cx| shell.dismiss_toast(id, cx)),
                tokens,
            )
        })
        .collect();
    toasts(cards, chrome::HEIGHT.mul_add(1.0, GAP), window)
}
