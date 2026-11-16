//! The words of a source report's standard rows, after Paseo usage/format.ts
//! `formatProviderUsageLabel` (16-27).
//!
//! [Decision Log] (2026-11-16, coordinator round-2 return, KO only)
//! - Purpose: a Korean reader sees "5시간 한도", not the node's "5-hour window".
//! - Constraint: protocol-26 reports carry the node's English label text
//!   (`vessel-node` `usage_reports/project.rs` names the saved-login windows,
//!   balances and details in a fixed vocabulary); a vendor's own names
//!   (models, features, plans, source names) arrive as given.
//! - Choice, as Paseo: a row is translated only when its id is a known
//!   standard id AND its label is exactly that id's English label;
//!   anything else passes through untouched, so a source that names a row
//!   itself is never renamed. A `code_review:` window whose label starts
//!   with "Code review · " translates the prefix and its period alike.
//! - Vessel's ids differ from Paseo's: Codex prefixes windows with
//!   `general:` (no label prefix) and Claude does not; both end in the
//!   period (`five_hour`, `weekly`, `unknown…`, `duration_<ms>`).
//! - Cost: the English labels are copied from the node's producer; if the
//!   node changes one, that row shows English until this table follows.
use crate::localization::Translator;

/// The Code review scope prefix the node writes before a period.
const REVIEW_ID: &str = "code_review:";
const REVIEW_LABEL: &str = "Code review · ";

/// A window's label in the viewer's language when it is a standard one.
#[must_use]
pub fn window_label_text(id: &str, label: &str, words: &'static Translator) -> String {
    let text = words.usage();
    if let (Some(period_id), Some(period_label)) =
        (id.strip_prefix(REVIEW_ID), label.strip_prefix(REVIEW_LABEL))
    {
        let period = period_text(period_id, period_label, words);
        return text
            .scoped_window(&period, text.code_review())
            .unwrap_or_else(|_| label.to_owned());
    }
    period_text(id.strip_prefix("general:").unwrap_or(id), label, words)
}

fn period_text(id: &str, label: &str, words: &'static Translator) -> String {
    let text = words.usage();
    match (id, label) {
        ("five_hour", "5-hour window") => text.five_hour().to_owned(),
        ("weekly", "Weekly window") => text.weekly().to_owned(),
        (unknown, "Window duration not reported") if unknown.starts_with("unknown") => {
            text.unknown_window().to_owned()
        }
        _ => id
            .strip_prefix("duration_")
            .filter(|milliseconds| label == format!("Reported window ({milliseconds} ms)"))
            .and_then(|milliseconds| text.duration_window(milliseconds).ok())
            .unwrap_or_else(|| label.to_owned()),
    }
}

/// The short label the summary uses for a standard window the source gave
/// none for: Paseo's "5h" and "wk".
#[must_use]
pub fn window_short_text(id: &str, label: &str, words: &'static Translator) -> Option<String> {
    let text = words.usage();
    match (id.strip_prefix("general:").unwrap_or(id), label) {
        ("five_hour", "5-hour window") => Some(text.short_five_hour().to_owned()),
        ("weekly", "Weekly window") => Some(text.short_weekly().to_owned()),
        _ => None,
    }
}

/// A balance's label: the node's saved-login credits row.
#[must_use]
pub fn balance_label_text(id: &str, label: &str, words: &'static Translator) -> String {
    match (id, label) {
        ("credits", "Reported credits") => words.usage().credits().to_owned(),
        _ => label.to_owned(),
    }
}

/// A detail row's label and value (the node's saved-login facts).
#[must_use]
pub fn detail_text(
    id: &str,
    label: &str,
    value: &str,
    words: &'static Translator,
) -> (String, String) {
    let text = words.usage();
    let label = match (id, label) {
        ("extra_usage", "Additional paid use") => text.extra_usage(),
        ("has_credits", "Credits available") => text.has_credits(),
        ("unlimited", "Unlimited credits") => text.unlimited(),
        ("allowed", "Vendor allows use") => text.allowed(),
        ("limit_reached", "Vendor reports limit reached") => text.limit_reached(),
        _ => return (label.to_owned(), value.to_owned()),
    };
    let value = match (id, value) {
        ("extra_usage", "Enabled") => text.enabled(),
        ("extra_usage", "Disabled") => text.disabled(),
        (_, "Yes") if id != "extra_usage" => text.yes(),
        (_, "No") if id != "extra_usage" => text.no(),
        _ => value,
    };
    (label.to_owned(), value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::localization::{ENGLISH, Locale};

    fn ko() -> &'static Translator {
        Translator::get(Locale::from_tag("ko").unwrap()).unwrap()
    }
    fn en() -> &'static Translator {
        Translator::get(ENGLISH).unwrap()
    }

    #[test]
    fn standard_windows_read_in_the_viewer_language() {
        assert_eq!(
            window_label_text("five_hour", "5-hour window", ko()),
            "5시간 한도"
        );
        assert_eq!(
            window_label_text("general:weekly", "Weekly window", ko()),
            "주간 한도"
        );
        assert_eq!(
            window_label_text(
                "code_review:unknown",
                "Code review · Window duration not reported",
                ko()
            ),
            "코드 리뷰 · 한도 기간이 보고되지 않음"
        );
        assert_eq!(
            window_label_text(
                "general:duration_3600000",
                "Reported window (3600000 ms)",
                ko()
            ),
            "보고된 한도 (3600000ms)"
        );
        assert_eq!(
            window_label_text("five_hour", "5-hour window", en()),
            "5-hour window"
        );
    }

    #[test]
    fn a_source_s_own_names_pass_through() {
        // The label differs from the standard text: the source named it.
        assert_eq!(
            window_label_text("weekly", "Weekly (Opus)", ko()),
            "Weekly (Opus)"
        );
        // A feature scope keeps the vendor's name and its period, as Paseo.
        assert_eq!(
            window_label_text("feature:gpt:weekly", "GPT · Weekly window", ko()),
            "GPT · Weekly window"
        );
        // Prefix without the matching id stays as given.
        assert_eq!(
            window_label_text("five_hour", "Code review · 5-hour window", ko()),
            "Code review · 5-hour window"
        );
        assert_eq!(
            balance_label_text("bonus", "Reported credits", ko()),
            "Reported credits"
        );
        assert_eq!(
            detail_text("plan_note", "Allowed", "Yes", ko()),
            ("Allowed".to_owned(), "Yes".to_owned())
        );
    }

    #[test]
    fn saved_login_rows_read_in_the_viewer_language() {
        assert_eq!(
            balance_label_text("credits", "Reported credits", ko()),
            "보고된 크레딧"
        );
        assert_eq!(
            detail_text("extra_usage", "Additional paid use", "Enabled", ko()),
            ("추가 유료 사용".to_owned(), "허용됨".to_owned())
        );
        assert_eq!(
            detail_text("limit_reached", "Vendor reports limit reached", "No", ko()),
            (
                "공급자가 한도 도달을 보고함".to_owned(),
                "아니요".to_owned()
            )
        );
        assert_eq!(
            window_short_text("general:five_hour", "5-hour window", ko()).as_deref(),
            Some("5h")
        );
        assert_eq!(window_short_text("weekly", "Weekly (Opus)", ko()), None);
    }
}
