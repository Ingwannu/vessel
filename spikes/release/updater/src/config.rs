//! The settings a release build compiles in.
//!
//! The feed's address and the release key are build-time settings, not
//! source: the repository is public and names no host (ADR 0014), and a
//! development build simply has none and never updates. The release
//! script (`spikes/release/build.sh`) sets them:
//!
//! | variable                | what                                   |
//! |-------------------------|----------------------------------------|
//! | `VESSEL_BUILD_LABEL`    | the build's label (`0.0.0.0.1`)        |
//! | `VESSEL_UPDATE_FEED`    | the feed's base (`https://...`)        |
//! | `VESSEL_UPDATE_KEY`     | the release key's public base64 line   |
//! | `VESSEL_UPDATE_CHANNEL` | the channel, `preview` when unset      |
//!
//! They are read with `option_env!` here, in this crate, so the app that
//! links it needs no code of its own to carry them; Cargo rebuilds this
//! crate when one changes (`build.rs` declares them and refuses a value
//! that cannot parse, so a typo fails the build, not the owner's update).

use crate::{
    channel::{Channel, ChannelError},
    feed::{FeedUrl, FeedUrlError},
    key::{KeyError, PublicKey},
    label::{Label, LabelError},
    platform::Target,
    service::{EVERY, FIRST_AFTER, Settings},
};

/// The label compiled in; `None` for a development build.
#[must_use]
pub fn build_label() -> Option<Label> {
    option_env!("VESSEL_BUILD_LABEL").and_then(|text| text.parse().ok())
}

/// Why the compiled-in settings do not parse (`build.rs` refuses these at
/// build time, so this is only reached by a build that skipped it).
#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// `VESSEL_BUILD_LABEL` is not a label.
    #[error("the build's label does not parse")]
    Label(#[source] LabelError),
    /// `VESSEL_UPDATE_FEED` is not a feed's address.
    #[error("the build's feed address does not parse")]
    Feed(#[source] FeedUrlError),
    /// `VESSEL_UPDATE_KEY` is not a public key.
    #[error("the build's release key does not parse")]
    Key(#[source] KeyError),
    /// `VESSEL_UPDATE_CHANNEL` is not a channel.
    #[error("the build's channel does not parse")]
    Channel(#[source] ChannelError),
}

/// The updater's settings as this build compiled them, or `None` when it
/// does not update itself: no label, feed or key was given, or this is
/// not a system the app is released for.
///
/// # Errors
/// [`SettingsError`] when a setting was given but does not parse.
pub fn settings_from_build() -> Result<Option<Settings>, SettingsError> {
    let (Some(label), Some(feed), Some(key), Some(target)) = (
        option_env!("VESSEL_BUILD_LABEL"),
        option_env!("VESSEL_UPDATE_FEED"),
        option_env!("VESSEL_UPDATE_KEY"),
        Target::current(),
    ) else {
        return Ok(None);
    };
    let channel = match option_env!("VESSEL_UPDATE_CHANNEL") {
        Some(text) => text.parse().map_err(SettingsError::Channel)?,
        None => Channel::preview(),
    };
    Ok(Some(Settings {
        feed: FeedUrl::parse(feed).map_err(SettingsError::Feed)?,
        key: PublicKey::from_base64(key).map_err(SettingsError::Key)?,
        channel,
        running: label.parse().map_err(SettingsError::Label)?,
        target,
        every: EVERY,
        first_after: FIRST_AFTER,
    }))
}
