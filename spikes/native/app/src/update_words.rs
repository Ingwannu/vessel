//! One owner for updater state and its prepared translated projection. Keeping
//! these fields together shrinks Shell; desktop and browser use the same cache.

use vessel_app_kit::ui::SharedString;
use vessel_app_view::localization::{Number, Translator};
use vessel_app_view::updates::{BuildName, PreparedUpdate, Problem, Status};

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
use native::problem;

#[cfg(target_arch = "wasm32")]
fn problem(_error: &crate::update::Words) -> Problem {
    // The browser never enters Failed; keep a safe classification even if a
    // future browser adapter gains an explicit unsupported update outcome.
    Problem::Unsupported
}

use crate::update::{UpdateState, Updated, Updates};

/// The updater's runtime and facts remain authoritative across retranslation.
pub(crate) struct UpdateUi {
    pub(crate) runtime: Option<std::rc::Rc<Updates>>,
    pub(crate) state: UpdateState,
    #[cfg_attr(
        target_arch = "wasm32",
        expect(dead_code, reason = "a browser has no updater that accepts a check")
    )]
    pub(crate) asked: bool,
    pub(crate) announcement: Option<Updated>,
    words: Option<PreparedUpdate>,
    restart_notice: Option<RestartNotice>,
}

struct RestartNotice {
    words: std::sync::Arc<str>,
}

impl UpdateUi {
    pub(crate) fn new(catalog: &Translator) -> Self {
        let mut ui = Self {
            runtime: None,
            state: UpdateState::Idle,
            asked: false,
            announcement: None,
            words: None,
            restart_notice: None,
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
            // `retry_in` and `kept` (a download that broke off, ADR 0094's
            // resuming updater) are for the update screen to show.
            UpdateState::Failed { error, .. } => Status::Failed {
                problem: problem(error),
            },
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

    pub(crate) fn translate(&mut self, catalog: &'static Translator, notice: &mut Option<String>) {
        self.refresh(catalog);
        if let Some(restart) = self.restart_notice.as_mut() {
            let visible = notice.as_deref() == Some(restart.words.as_ref());
            restart.words = Problem::Restart.words(catalog).into();
            if visible {
                *notice = Some(restart.words.to_string());
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn restart_failed(&mut self, catalog: &Translator) -> std::sync::Arc<str> {
        let words: std::sync::Arc<str> = Problem::Restart.words(catalog).into();
        self.restart_notice = Some(RestartNotice {
            words: std::sync::Arc::clone(&words),
        });
        words
    }

    pub(crate) fn problem(&self) -> Option<SharedString> {
        self.words
            .as_ref()
            .and_then(PreparedUpdate::problem)
            .map(Into::into)
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

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn changing_language_rewords_only_the_restart_notice_it_still_owns() {
        use super::UpdateUi;
        use vessel_app_view::localization::{ENGLISH, Locale, Translator};

        let en = Translator::get(ENGLISH).unwrap();
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        let mut ui = UpdateUi::new(en);
        let mut visible = Some(ui.restart_failed(en).to_string());
        assert_eq!(
            visible.as_deref(),
            Some("The updated app could not be started. Vessel is still open; try restarting it.")
        );
        ui.translate(ko, &mut visible);
        assert_eq!(
            visible.as_deref(),
            Some(
                "업데이트된 앱을 시작하지 못했습니다. Vessel은 계속 열려 있으니 다시 시작해 보세요."
            )
        );
        visible = Some("An unrelated notice".to_owned());
        ui.translate(en, &mut visible);
        assert_eq!(visible.as_deref(), Some("An unrelated notice"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn starting_a_new_check_clears_the_previous_update_problem() {
        use super::UpdateUi;
        use crate::update::UpdateState;
        use vessel_app_view::localization::{ENGLISH, Translator};

        let catalog = Translator::get(ENGLISH).unwrap();
        let mut ui = UpdateUi::new(catalog);
        ui.state = UpdateState::Failed {
            error: std::sync::Arc::new(vessel_app_updater::UpdateError::Interrupted),
            retry_in: None,
            kept: None,
        };
        ui.refresh(catalog);
        assert_eq!(
            ui.problem().as_deref(),
            Some("The update was interrupted. Try again.")
        );
        ui.state = UpdateState::Checking;
        ui.refresh(catalog);
        assert!(ui.problem().is_none());
    }
}
