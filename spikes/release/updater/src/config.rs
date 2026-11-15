//! The settings a release build carries.
//!
//! The feed's address and the release key are release settings, not
//! source: the repository is public and names no host (ADR 0014), and a
//! development build simply has none and never updates. The release
//! script (`spikes/release/build.sh`) gives them:
//!
//! | setting                 | how it gets into the app                    |
//! |-------------------------|---------------------------------------------|
//! | `VESSEL_BUILD_LABEL`    | compiled in: the build's label (`0.0.0.0.1`)|
//! | `VESSEL_UPDATE_KEY`     | compiled in: the release key's public line  |
//! | `VESSEL_UPDATE_CHANNEL` | compiled in: the channel, `preview` unset   |
//! | the feed's address      | patched in after compiling ([`FEED_SLOT_LEN`]) |
//!
//! The three compiled in are read with `option_env!` here, in this crate,
//! so the app that links it needs no code of its own to carry them; Cargo
//! rebuilds this crate when one changes (`build.rs` declares them and
//! refuses a value that cannot parse, so a typo fails the build, not the
//! owner's update).
//!
//! The feed's address is not compiled in (ADR 0099). It carries the
//! feed's token, and the machine that compiles a release holds no secret
//! (the operator's rule since 2026-11-15: no compiler runs on the machine
//! that holds the token and the signing key). The app is compiled with a
//! fixed slot of [`FEED_SLOT_LEN`] bytes holding a marker, and the
//! machine that signs the release writes the address into exactly those
//! bytes of the built executable (`spikes/release/tools/patch-feed.py`),
//! before it packages, sums and signs. `build.rs` refuses
//! `VESSEL_UPDATE_FEED`, the variable that used to compile the address
//! in, so no build can carry it the old way by mistake.

use crate::{
    channel::{Channel, ChannelError},
    feed::{FeedUrl, FeedUrlError},
    key::{KeyError, PublicKey},
    label::{Label, LabelError},
    platform::Target,
    service::{EVERY, FIRST_AFTER, RETRY_AFTER, Settings},
};

/// The size of the feed address's slot, in bytes: the longest address a
/// release can write in is one byte shorter (a written slot ends in at
/// least one NUL). `tools/patch-feed.py` carries the same number and the
/// same marker; its tests read both from this file, so the two cannot
/// drift apart unnoticed.
///
/// [Decision Log, ADR 0099]
/// - Purpose: a release compiled on a machine without secrets must still
///   carry the feed's address, which holds a token.
/// - Constraint: the address must be readable at run time from bytes
///   written after linking, so the optimiser must not fold the marker
///   into the code (it is an immutable static with a known value), and
///   the marker must occur exactly once in the executable so the patch
///   finds one place only.
/// - Alternatives: compile on the signing machine (the operator refused
///   it); give the build machine the token (it holds no secret by rule);
///   read the address from a file beside the app at run time (a second
///   file to install, update and keep out of reach of other users, and an
///   app that loses its feed when the file is lost).
/// - Chosen: a fixed-length slot, read through [`std::hint::black_box`]
///   (the workspace forbids `unsafe`, so not `read_volatile`), patched in
///   the built file.
/// - Cost: `black_box` is documented as best effort; the release
///   procedure therefore runs every patched Linux build against a test
///   feed before the real address goes in (`tools/feed-slot-proof.sh`),
///   which fails if the optimiser ever folds the read.
pub(crate) const FEED_SLOT_LEN: usize = 512;

/// The slot's first bytes while nothing is written in. No byte of the
/// marker is NUL, which is how the reader tells an unwritten slot.
const FEED_SLOT_HEADER: &[u8] = b"VESSEL-UPDATE-FEED-SLOT/ADR-0099/UNPATCHED/";

/// What fills the rest of an unwritten slot.
const FEED_SLOT_FILL: u8 = b'#';

/// The slot itself. Only its address leaves this module, through
/// `black_box`, so the bytes are read from the executable at run time.
static FEED_SLOT: [u8; FEED_SLOT_LEN] = unwritten_slot();

/// The marker: [`FEED_SLOT_HEADER`] and then [`FEED_SLOT_FILL`]. Built at
/// compile time only, so its bytes exist in the executable once, as the
/// static's value.
const fn unwritten_slot() -> [u8; FEED_SLOT_LEN] {
    let mut slot = [FEED_SLOT_FILL; FEED_SLOT_LEN];
    let (head, _fill) = slot.split_at_mut(FEED_SLOT_HEADER.len());
    head.copy_from_slice(FEED_SLOT_HEADER);
    slot
}

/// The label compiled in; `None` for a development build.
#[must_use]
pub fn build_label() -> Option<Label> {
    option_env!("VESSEL_BUILD_LABEL").and_then(|text| text.parse().ok())
}

/// Why the build's settings cannot be used (`build.rs` refuses the
/// compiled-in ones at build time, so those are only reached by a build
/// that skipped it).
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// `VESSEL_BUILD_LABEL` is not a label.
    #[error("the build's label does not parse")]
    Label(#[source] LabelError),
    /// The feed's address written into the build is not a feed's address.
    #[error("the build's feed address does not parse")]
    Feed(#[source] FeedUrlError),
    /// The build carries a release key, but its feed address slot was
    /// never written: it was compiled for a release and not packaged.
    #[error("the build's feed address was never written in (ADR 0099)")]
    FeedNotWritten,
    /// The slot holds bytes no patch writes (text after the padding, no
    /// text at all, or not UTF-8).
    #[error("the build's feed address slot holds something other than an address")]
    FeedSlot,
    /// `VESSEL_UPDATE_KEY` is not a public key.
    #[error("the build's release key does not parse")]
    Key(#[source] KeyError),
    /// `VESSEL_UPDATE_CHANNEL` is not a channel.
    #[error("the build's channel does not parse")]
    Channel(#[source] ChannelError),
}

/// The text in a slot: `None` for an unwritten one (no NUL anywhere: the
/// marker holds none), the address for a written one (text, then NUL to
/// the end).
fn slot_text(slot: &[u8]) -> Result<Option<&str>, SettingsError> {
    let Some(end) = slot.iter().position(|&byte| byte == 0) else {
        return Ok(None);
    };
    let (text, padding) = slot.split_at_checked(end).ok_or(SettingsError::FeedSlot)?;
    if text.is_empty() || padding.iter().any(|&byte| byte != 0) {
        return Err(SettingsError::FeedSlot);
    }
    std::str::from_utf8(text)
        .map(Some)
        .map_err(|_not_text| SettingsError::FeedSlot)
}

/// The feed's address written into this executable, or `None` when the
/// slot is unwritten.
fn written_feed() -> Result<Option<FeedUrl>, SettingsError> {
    // Through `black_box`: the compiler must not assume the slot still
    // holds the marker it was compiled with (see FEED_SLOT_LEN).
    let slot: &[u8; FEED_SLOT_LEN] = std::hint::black_box(&FEED_SLOT);
    slot_text(slot)?
        .map(|text| FeedUrl::parse(text).map_err(SettingsError::Feed))
        .transpose()
}

/// The updater's settings as this build carries them, or `None` when it
/// does not update itself: no label or key was compiled in, or this is
/// not a system the app is released for.
///
/// # Errors
/// [`SettingsError`] when a setting was given but cannot be used,
/// including a release build whose feed address was never written in.
pub fn settings_from_build() -> Result<Option<Settings>, SettingsError> {
    let (Some(label), Some(key), Some(target)) = (
        option_env!("VESSEL_BUILD_LABEL"),
        option_env!("VESSEL_UPDATE_KEY"),
        Target::current(),
    ) else {
        return Ok(None);
    };
    let feed = written_feed()?.ok_or(SettingsError::FeedNotWritten)?;
    let channel = match option_env!("VESSEL_UPDATE_CHANNEL") {
        Some(text) => text.parse().map_err(SettingsError::Channel)?,
        None => Channel::preview(),
    };
    Ok(Some(Settings {
        feed,
        key: PublicKey::from_base64(key).map_err(SettingsError::Key)?,
        channel,
        running: label.parse().map_err(SettingsError::Label)?,
        target,
        every: EVERY,
        first_after: FIRST_AFTER,
        automatic: true,
        retry_after: RETRY_AFTER.to_vec(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A slot as `tools/patch-feed.py` writes it.
    fn written(address: &str) -> [u8; FEED_SLOT_LEN] {
        let mut slot = [0; FEED_SLOT_LEN];
        slot[..address.len()].copy_from_slice(address.as_bytes());
        slot
    }

    #[test]
    fn the_unwritten_slot_is_the_marker_and_holds_no_nul() {
        let slot = unwritten_slot();
        assert_eq!(slot.len(), FEED_SLOT_LEN);
        assert!(slot.starts_with(FEED_SLOT_HEADER));
        assert!(
            slot[FEED_SLOT_HEADER.len()..]
                .iter()
                .all(|&byte| byte == FEED_SLOT_FILL)
        );
        assert!(!slot.contains(&0));
        assert_eq!(FEED_SLOT, slot);
    }

    #[test]
    fn an_unwritten_slot_means_no_feed() {
        assert_eq!(slot_text(&FEED_SLOT).unwrap(), None);
        assert!(written_feed().unwrap().is_none());
    }

    #[test]
    fn a_written_slot_gives_its_address() {
        let address = "https://feed.example.test/token-123/";
        let slot = written(address);
        assert_eq!(slot_text(&slot).unwrap(), Some(address));
        let longest = "h".repeat(FEED_SLOT_LEN - 1);
        assert_eq!(
            slot_text(&written(&longest)).unwrap(),
            Some(longest.as_str())
        );
    }

    #[test]
    fn what_no_patch_writes_is_refused() {
        // Text after the padding: a slot written twice, or a wrong range.
        let mut twice = written("https://a.example.test/");
        twice[100] = b'x';
        assert!(matches!(slot_text(&twice), Err(SettingsError::FeedSlot)));
        // No text at all.
        assert!(matches!(
            slot_text(&[0; FEED_SLOT_LEN]),
            Err(SettingsError::FeedSlot)
        ));
        // Not UTF-8.
        let mut bytes = written("https://a.example.test/");
        bytes[3] = 0xff;
        assert!(matches!(slot_text(&bytes), Err(SettingsError::FeedSlot)));
    }

    #[test]
    fn a_written_address_is_checked_as_a_feed_address() {
        let plain = written("http://feed.example.test/");
        let text = slot_text(&plain).unwrap().unwrap();
        assert!(matches!(FeedUrl::parse(text), Err(FeedUrlError::Scheme)));
    }
}
