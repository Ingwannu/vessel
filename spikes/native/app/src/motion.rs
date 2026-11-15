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
//!
//! One flag for every reason motion is held (the coordinator, 2026-11-14,
//! word 39: "off, reduced motion and a software renderer = the end state
//! at once"): the system's preference, a software renderer and the app's
//! own Animations setting are each kept here and the flag is their "or"
//! ([`apply`]). Everything that moves reads `cx.reduce_motion()`: GPUI's
//! animations end at once under it, the kit's controls hold still, and the
//! text caret stops blinking (the patched gpui-base,
//! `spikes/release/patches/README.md`).

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::App;

/// Whether the window draws on a software renderer ([`note_renderer`]).
static SOFTWARE: AtomicBool = AtomicBool::new(false);
/// The system's reduced motion, as last read ([`follow`]).
static SYSTEM: AtomicBool = AtomicBool::new(false);
/// The app's Animations setting is off ([`animations`]).
static OFF: AtomicBool = AtomicBool::new(false);

/// Set GPUI's flag from the reasons kept here; a forced value wins.
fn apply(cx: &mut App) {
    // A forced value fixes the system's and the renderer's reasons for
    // this run (a capture on a virtual display, which draws in software);
    // the person's own Animations setting still applies under a forced
    // "0", so a frame strip can show it on and off.
    let off = OFF.load(Ordering::Relaxed);
    let held = match forced() {
        Some(true) => true,
        Some(false) => off,
        None => SYSTEM.load(Ordering::Relaxed) || SOFTWARE.load(Ordering::Relaxed) || off,
    };
    cx.set_reduce_motion(held);
}

/// Settings › Appearance › Animations (track S3's setting, word 39): off
/// holds every motion as reduced motion does.
pub(crate) fn animations(on: bool, cx: &mut App) {
    OFF.store(!on, Ordering::Relaxed);
    apply(cx);
}

/// `VESSEL_APP_REDUCE_MOTION=1` (or `0`): the flag fixed for this run and
/// the system not followed, as `VESSEL_APP_THEME` fixes the scheme, so a
/// capture on a virtual display can show the held-still state without
/// changing the desktop's settings (track S2's request). Not a setting:
/// nothing is stored.
fn forced() -> Option<bool> {
    match std::env::var("VESSEL_APP_REDUCE_MOTION").as_deref() {
        Ok("1") => Some(true),
        Ok("0") => Some(false),
        Ok(_) | Err(_) => None,
    }
}

/// Follow the system's preference from now on (or the forced value).
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
pub(crate) fn follow(cx: &mut App) {
    use ashpd::desktop::settings::{ReducedMotion, Settings};
    use futures_lite::StreamExt as _;

    const GNOME: &str = "org.gnome.desktop.interface";
    const ANIMATIONS: &str = "enable-animations";

    if forced().is_some() {
        apply(cx);
        return;
    }
    cx.spawn(async move |cx| {
        let Ok(settings) = Settings::new().await else {
            tracing::debug!("no desktop portal: reduce motion stays off");
            return;
        };
        let mut set = |reduce: bool| {
            SYSTEM.store(reduce, Ordering::Relaxed);
            cx.update(apply);
        };
        // Each source is subscribed to before it is read, so a change
        // between the two is in the stream and the last value applied is
        // the newest (track V1's review: read-then-subscribe could miss it
        // and keep a stale flag until the next change).
        if let Ok(changes) = settings.receive_reduced_motion_changed().await
            && let Ok(motion) = settings.reduced_motion().await
        {
            let reduced = |motion: ReducedMotion| motion == ReducedMotion::ReducedMotion;
            latest(reduced(motion), changes.map(reduced), &mut set).await;
            return;
        }
        if let Ok(changes) = settings
            .receive_setting_changed_with_args::<bool>(GNOME, ANIMATIONS)
            .await
            && let Ok(animations) = settings.read::<bool>(GNOME, ANIMATIONS).await
        {
            let changes = changes
                .filter_map(Result::ok)
                .map(|animations: bool| !animations);
            latest(!animations, changes, &mut set).await;
        }
    })
    .detach();
}

/// Apply `first` (read after subscribing), then every change in order: the
/// last applied is the newest whatever arrived between subscribing and
/// reading.
#[cfg(any(target_os = "linux", target_os = "freebsd"))]
async fn latest(
    first: bool,
    changes: impl futures_lite::Stream<Item = bool>,
    set: &mut impl FnMut(bool),
) {
    use futures_lite::StreamExt as _;

    set(first);
    let mut changes = std::pin::pin!(changes);
    while let Some(reduce) = changes.next().await {
        set(reduce);
    }
}

/// Elsewhere the flag stays as GPUI starts it (off) for now, unless
/// forced.
#[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
pub(crate) fn follow(cx: &mut App) {
    apply(cx);
}

#[cfg(all(test, any(target_os = "linux", target_os = "freebsd")))]
mod tests {
    use super::latest;

    /// A change made between subscribing and reading arrives in the stream
    /// after the read value, so the flag ends at the newest value; a read
    /// that already saw the change is followed by the same value, which
    /// changes nothing (track V1's review).
    #[test]
    fn the_last_value_applied_is_the_newest() {
        for (first, changes, newest) in [
            // Read before the change landed: the stream brings it.
            (false, vec![true], true),
            // Read after it landed: the stream repeats it.
            (true, vec![true], true),
            // Two changes in the window, then one later.
            (false, vec![true, false, true], true),
            // No change at all.
            (true, vec![], true),
        ] {
            let mut applied = Vec::new();
            futures_lite::future::block_on(latest(
                first,
                futures_lite::stream::iter(changes.clone()),
                &mut |reduce| applied.push(reduce),
            ));
            assert_eq!(applied.last(), Some(&newest), "{first} then {changes:?}");
            assert_eq!(applied.len(), changes.len() + 1);
        }
    }
}

/// Note the window's renderer: on a software one (llvmpipe, lavapipe,
/// Windows' basic render driver) the whole app moves as under reduced
/// motion. Every frame there is the whole window drawn by the processor:
/// on 0.0.0.0.13 the board's working spinners cost about five cores and
/// the effort card's field three (the coordinator's measure and the lead's,
/// release build on a virtual display, 2026-11-14). GPUI's animations and
/// the kit's controls hold still under the flag; the colours still change;
/// the caret is steady (its blink cost about 31 % of a core at 1280 × 800
/// and 62 % at 1920 × 1200 on 0.0.0.0.14, the coordinator's measure).
/// `VESSEL_APP_REDUCE_MOTION=0` keeps the motion, to measure it.
pub(crate) fn note_renderer(window: &gpui::Window, cx: &mut App) {
    if forced() == Some(false) {
        return;
    }
    let software = window
        .gpu_specs()
        .is_some_and(|specs| specs.is_software_emulated);
    if software {
        SOFTWARE.store(true, Ordering::Relaxed);
        apply(cx);
        tracing::info!("a software renderer: the app holds its motion still");
    }
}

#[cfg(test)]
mod token_tests {
    use vessel_app_kit::motion::{QUICK, STANDARD, ease};
    use vessel_app_view::slider::motion::{DRAG_GLIDE, GLIDE, SLIDE};

    /// The effort slider's model (`view`, which cannot read the kit)
    /// restates the kit's motion tokens; this keeps them one set.
    #[test]
    fn the_slider_moves_by_the_kit_tokens() {
        assert!((DRAG_GLIDE - QUICK.as_secs_f32()).abs() < 1e-6);
        assert!((GLIDE - STANDARD.as_secs_f32()).abs() < 1e-6);
        for step in 0..=20_u8 {
            let t = f32::from(step) / 20.0;
            assert!((SLIDE.at(t) - ease(t)).abs() < 1e-3, "at {t}");
        }
    }
}
