//! The system's "reduce motion" preference, kept in GPUI's
//! `App::reduce_motion`: with it set, GPUI's animations (`AnimationExt`,
//! the kit's spinner) hold still instead of moving.
//!
//! [Decision Log]
//! - Purpose: GPUI has the flag and its animations honour it, but nothing
//!   set it: the pinned platform crates do not read the system's
//!   preference, and the app did not (track S2's question, 2026-11-13).
//! - Linux and FreeBSD: the desktop portal's `org.freedesktop.appearance`
//!   `reduced-motion` (the portal's own key), read at start and followed
//!   as it changes; where a portal does not have that key, GNOME's
//!   `org.gnome.desktop.interface` `enable-animations` (off = reduce),
//!   which the portal relays, read the same way. No portal: the flag
//!   stays off, as before.
//! - Alternatives: a setting of the app's own (Paseo has none; the
//!   system's is the person's choice already); reading GNOME's dconf
//!   directly (desktop-specific, and a sandboxed build cannot).
//! - Not here yet: Windows (`SPI_GETCLIENTAREAANIMATION` needs an
//!   `unsafe` call this workspace forbids; where it goes is the
//!   coordinator's to decide) and macOS (not built).
//! - Cost: one portal connection on a task of its own, as GPUI's own
//!   colour-scheme follower keeps.

use gpui::App;

/// Follow the system's preference from now on.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) fn follow(cx: &mut App) {
    use ashpd::desktop::settings::{ReducedMotion, Settings};
    use futures_lite::StreamExt as _;

    const GNOME: &str = "org.gnome.desktop.interface";
    const ANIMATIONS: &str = "enable-animations";

    cx.spawn(async move |cx| {
        let Ok(settings) = Settings::new().await else {
            tracing::debug!("no desktop portal: reduce motion stays off");
            return;
        };
        let set = |reduce: bool, cx: &mut gpui::AsyncApp| {
            cx.update(|cx| cx.set_reduce_motion(reduce));
        };
        if let Ok(motion) = settings.reduced_motion().await {
            set(motion == ReducedMotion::ReducedMotion, cx);
            let Ok(mut changes) = settings.receive_reduced_motion_changed().await else {
                return;
            };
            while let Some(motion) = changes.next().await {
                set(motion == ReducedMotion::ReducedMotion, cx);
            }
        } else if let Ok(animations) = settings.read::<bool>(GNOME, ANIMATIONS).await {
            set(!animations, cx);
            let Ok(mut changes) = settings
                .receive_setting_changed_with_args::<bool>(GNOME, ANIMATIONS)
                .await
            else {
                return;
            };
            while let Some(Ok(animations)) = changes.next().await {
                set(!animations, cx);
            }
        }
    })
    .detach();
}

/// Elsewhere the flag stays as GPUI starts it (off) for now.
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
pub(crate) const fn follow(_cx: &mut App) {}
