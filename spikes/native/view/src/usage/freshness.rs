//! Only a successful observation owns freshness; retry/query instants never do.
use super::SourceView;
use crate::localization::Translator;
use chrono::{DateTime, Utc};

const MINUTE_SECONDS: i64 = 60;
const HOUR_SECONDS: i64 = 3_600;
const DAY_SECONDS: i64 = 86_400;

/// Minute-tier redraws can update this label without fetching an external source.
#[must_use]
pub fn source_freshness(
    view: &SourceView<'_>,
    now: DateTime<Utc>,
    words: &'static Translator,
) -> String {
    let text = words.usage();
    let Some(observed) = view.observed else {
        return text.never_updated().to_owned();
    };
    let seconds = now.signed_duration_since(observed.at).num_seconds().max(0);
    let age = if seconds < MINUTE_SECONDS {
        text.fresh_just_now().to_owned()
    } else if seconds < HOUR_SECONDS {
        text.fresh_minutes(&seconds.div_euclid(MINUTE_SECONDS).to_string())
            .unwrap_or_else(|_| text.none().to_owned())
    } else if seconds < DAY_SECONDS {
        text.fresh_hours(&seconds.div_euclid(HOUR_SECONDS).to_string())
            .unwrap_or_else(|_| text.none().to_owned())
    } else {
        text.fresh_days(&seconds.div_euclid(DAY_SECONDS).to_string())
            .unwrap_or_else(|_| text.none().to_owned())
    };
    if view.retained {
        text.last_updated(&age)
    } else {
        text.updated(&age)
    }
    .unwrap_or_else(|_| text.none().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        localization::{ENGLISH, Locale},
        usage::source_view,
    };
    use vessel_proto::usage_reports::{
        AvailableUsageReport, SourceReportState, SuccessfulUsageReport, UsageFetchFailure,
    };
    #[test]
    fn minute_hour_day_boundaries_floor_and_failures_keep_the_success_clock() {
        let en = Translator::get(ENGLISH).unwrap();
        let observed = SuccessfulUsageReport {
            at: DateTime::UNIX_EPOCH,
            report: AvailableUsageReport::default(),
        };
        let state = SourceReportState::Available {
            observed: observed.clone(),
        };
        let view = source_view(&state, en);
        for (seconds, expected) in [
            (59, "Updated \u{2068}just now\u{2069}"),
            (60, "Updated \u{2068}\u{2068}1\u{2069}m ago\u{2069}"),
            (3_599, "Updated \u{2068}\u{2068}59\u{2069}m ago\u{2069}"),
            (3_600, "Updated \u{2068}\u{2068}1\u{2069}h ago\u{2069}"),
            (86_399, "Updated \u{2068}\u{2068}23\u{2069}h ago\u{2069}"),
            (86_400, "Updated \u{2068}\u{2068}1\u{2069}d ago\u{2069}"),
        ] {
            let now = DateTime::from_timestamp(seconds, 0).unwrap();
            assert_eq!(source_freshness(&view, now, en), expected);
        }
        let failed = SourceReportState::Failed {
            attempted_at: Some(DateTime::from_timestamp(86_400, 0).unwrap()),
            reason: UsageFetchFailure::Unreachable,
            retry_at: None,
            retained: Some(observed),
            logins: Default::default(),
        };
        let now = DateTime::from_timestamp(86_400, 0).unwrap();
        assert_eq!(
            source_freshness(&source_view(&failed, en), now, en),
            "Last updated \u{2068}\u{2068}1\u{2069}d ago\u{2069}"
        );
        let ko = Translator::get(Locale::from_tag("ko").unwrap()).unwrap();
        assert_eq!(
            source_freshness(&source_view(&failed, ko), now, ko),
            "마지막 업데이트: \u{2068}\u{2068}1\u{2069}일 전\u{2069}"
        );
        assert_eq!(
            source_freshness(&source_view(&SourceReportState::NotRequested, en), now, en),
            en.usage().never_updated()
        );
    }
}
