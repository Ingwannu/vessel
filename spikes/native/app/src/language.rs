//! Language selection at the host edge; changing it refreshes existing fields.
//!
//! [Decision Log]
//! - Purpose: the saved choice applies before the first frame and at once on
//!   every open screen; `VESSEL_APP_LANG` remains a checking override.
//! - Constraint: view has no I/O, while cached fields need a Window to change.
//! - Alternatives: restarting loses current work; resolving in Render parses
//!   host settings and can initialize a catalog during a frame.
//! - Choice: resolve and prepare at startup or at a preference change, then
//!   replace the shell's borrowed catalog and legacy table together.
//! - Cost: unmigrated areas still use English or Korean tables until moved.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::Lang;
use vessel_app_view::localization::{InitError, LanguagePreference, Translator, resolve_languages};

use crate::shell::Shell;

/// The checking override is captured at startup. Preferred system tags are
/// read at selection events, never in Render; sys-locale uses navigator's full
/// language list on web, matching the native providers' preference order.
pub(crate) struct Language {
    checking: Option<String>,
}

impl Language {
    pub(crate) fn new() -> Self {
        Self {
            checking: std::env::var("VESSEL_APP_LANG").ok(),
        }
    }

    pub(crate) fn prepare(
        &self,
        preference: LanguagePreference,
    ) -> Result<&'static Translator, InitError> {
        Translator::get(resolve_languages(
            preference,
            sys_locale::get_locales(),
            self.checking.as_deref(),
        ))
    }
}

type RefreshWords = fn(&mut Shell, &mut Window, &mut Context<'_, Shell>);

// A retained translated placeholder has one refresh method. Register it here
// so changing language never needs to navigate to or reset a cached form.
const PLACEHOLDERS: &[RefreshWords] = &[
    crate::areas::import::refresh_words,
    crate::areas::draft::refresh_words,
    |shell, window, cx| crate::areas::pair::refresh_words(shell, window, cx),
    |shell, window, cx| {
        shell
            .areas
            .board
            .query
            .set_placeholder(shell.t.lang.board().archive_search, window, cx);
    },
    |shell, window, cx| shell.refresh_find_words(window, cx),
    Shell::refresh_question_words,
    |shell, window, cx| shell.refresh_queue_words(window, cx),
    |shell, window, cx| shell.refresh_project_words(window, cx),
];

impl Shell {
    /// Refresh labels held by controls as well as labels read by renderers.
    /// Text typed by the person, focus, selection and the route stay intact.
    pub(crate) fn apply_language(
        &mut self,
        words: &'static Translator,
        window: &mut Window,
        cx: &mut Context<'_, Self>,
    ) {
        if std::ptr::eq(self.words, words) {
            return;
        }
        self.words = words;
        self.t = Lang::from_tag(words.locale().tag()).text();
        self.refresh_general_words();
        self.refresh_preference_refusal();
        crate::areas::settings::apply_composer(self, window, cx);
        for refresh in PLACEHOLDERS {
            refresh(self, window, cx);
        }
        crate::areas::pair::translate_outcomes(self);
        self.updates.translate(self.words, &mut self.notice);
        self.translate_rewind_notice();
        self.translate_fork_words();
        let _changed = self.schedules_changed(cx);
        self.translate_diagnostic_notice();
        self.areas
            .conversation
            .list
            .changed(0..self.areas.conversation.list.rows());
        self.areas.session.entries.remeasure();
        cx.notify();
    }
}
