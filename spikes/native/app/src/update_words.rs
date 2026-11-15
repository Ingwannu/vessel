//! One owner for updater state and its prepared translated projection. Keeping
//! these fields together shrinks Shell; desktop and browser use the same cache.

use vessel_app_kit::ui::SharedString;
use vessel_app_view::localization::{Number, Translator};
use vessel_app_view::updates::{BuildName, PreparedUpdate, Status};

use crate::update::{UpdateState, Updated, Updates};

/// The updater's runtime and facts remain authoritative across retranslation.
pub(crate) struct UpdateUi {
    pub(crate) runtime: Option<Updates>,
    pub(crate) state: UpdateState,
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "a browser has no updater that accepts a check")
    )]
    pub(crate) asked: bool,
    pub(crate) announcement: Option<Updated>,
    words: Option<PreparedUpdate>,
}

impl UpdateUi {
    pub(crate) fn new(catalog: &Translator) -> Self {
        let mut ui = Self {
            runtime: None,
            state: UpdateState::Idle,
            asked: false,
            announcement: None,
            words: None,
        };
        ui.refresh(catalog);
        ui
    }

    pub(crate) fn refresh(&mut self, catalog: &Translator) {
        let status = match &self.state {
            UpdateState::Idle => Status::Idle,
            UpdateState::Checking => Status::Checking,
            UpdateState::UpToDate { .. } => Status::Current,
            UpdateState::Downloading {
                label,
                received,
                total,
                ..
            } => Status::Downloading {
                build: BuildName::from(label.to_string().as_str()),
                progress: Number::count(u32::from(percent(received.get(), total.get()))),
            },
            UpdateState::Ready { label, .. } => Status::Ready {
                build: BuildName::from(label.to_string().as_str()),
            },
            UpdateState::Failed { .. } => Status::Failed,
        };
        let announced = self
            .announcement
            .as_ref()
            .map(|note| BuildName::from(note.label.as_str()));
        self.words = match PreparedUpdate::new(
            catalog,
            &BuildName::from(crate::LABEL),
            &status,
            announced.as_ref(),
        ) {
            Ok(words) => Some(words),
            Err(error) => {
                tracing::warn!(%error, "update messages could not be prepared");
                None
            }
        };
    }

    pub(crate) fn version(&self) -> SharedString {
        self.words
            .as_ref()
            .map_or_else(|| crate::LABEL.into(), |words| words.version().into())
    }

    pub(crate) fn status(&self, catalog: &'static Translator) -> SharedString {
        self.words.as_ref().map_or_else(
            || catalog.update().details_unavailable().into(),
            |words| words.status().into(),
        )
    }

    pub(crate) fn action(&self, catalog: &'static Translator) -> SharedString {
        self.words.as_ref().map_or_else(
            || catalog.update().update().into(),
            |words| words.action().into(),
        )
    }

    pub(crate) fn progress(&self, catalog: &'static Translator) -> SharedString {
        self.words
            .as_ref()
            .and_then(PreparedUpdate::progress)
            .map_or_else(|| catalog.update().details_unavailable().into(), Into::into)
    }

    pub(crate) fn banner(&self, catalog: &'static Translator) -> SharedString {
        self.words
            .as_ref()
            .and_then(PreparedUpdate::banner)
            .map_or_else(|| catalog.update().details_unavailable().into(), Into::into)
    }
}

/// One percentage bound for native and browser byte-count projections.
const COMPLETE_PERCENT: u8 = 100;

fn percent(received: u64, total: u64) -> u8 {
    if total == 0 {
        return 0;
    }
    let percent = received
        .saturating_mul(u64::from(COMPLETE_PERCENT))
        .div_euclid(total);
    u8::try_from(percent.min(u64::from(COMPLETE_PERCENT))).unwrap_or(COMPLETE_PERCENT)
}

#[cfg(test)]
mod tests {
    use super::{COMPLETE_PERCENT, percent};

    #[test]
    fn download_progress_keeps_the_existing_zero_floor_and_clamp() {
        assert_eq!(percent(1, 0), 0);
        assert_eq!(percent(1, 3), COMPLETE_PERCENT.div_euclid(3));
        assert_eq!(percent(1, 1), COMPLETE_PERCENT);
        assert_eq!(percent(2, 1), COMPLETE_PERCENT);
    }
}
