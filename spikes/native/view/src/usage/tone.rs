//! Quota colour follows reported consumption, regardless of used/remaining display.
use vessel_proto::conversation::FiniteNumber;

/// Paseo's source override and fallback share the four semantic states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// No source tone; ordinary low consumption.
    Default,
    /// Provider explicitly reported a healthy balance.
    Ok,
    /// Consumption reached the warning threshold.
    Warning,
    /// Consumption exceeded the danger threshold.
    Danger,
}
const WARNING_PERCENT: f64 = 70.0;
const DANGER_PERCENT: f64 = 90.0;

/// Sources may override tone; fallback uses used share, never remaining share.
#[must_use]
pub fn window_tone(used: Option<FiniteNumber>, reported: Option<Tone>) -> Tone {
    if let Some(tone) = reported {
        return tone;
    }
    match used {
        Some(used) if used.get() > DANGER_PERCENT => Tone::Danger,
        Some(used) if used.get() >= WARNING_PERCENT => Tone::Warning,
        Some(_) | None => Tone::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_thresholds_and_source_overrides_follow_paseo() {
        let number = |value| Some(FiniteNumber::new(value).unwrap());
        assert_eq!(window_tone(None, None), Tone::Default);
        assert_eq!(
            window_tone(number(WARNING_PERCENT.mul_add(1.0, -0.1)), None),
            Tone::Default
        );
        assert_eq!(window_tone(number(WARNING_PERCENT), None), Tone::Warning);
        assert_eq!(window_tone(number(DANGER_PERCENT), None), Tone::Warning);
        assert_eq!(
            window_tone(number(DANGER_PERCENT.mul_add(1.0, 0.1)), None),
            Tone::Danger
        );
        assert_eq!(
            window_tone(number(DANGER_PERCENT), Some(Tone::Ok)),
            Tone::Ok
        );
        assert_eq!(window_tone(None, Some(Tone::Danger)), Tone::Danger);
    }
}
