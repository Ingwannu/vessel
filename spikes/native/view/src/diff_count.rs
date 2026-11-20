//! A diff count in the page's compact form (`formatDiffCount`,
//! `web/src/data/workspace/diff-stat.ts`): plain under 1000, otherwise one
//! decimal and `k` / `m` / `b`, trailing `.0` dropped, lower case.

/// `value` as that compact count.
#[must_use]
pub fn format_diff_count(value: u32) -> String {
    let value = u64::from(value);
    let (unit, suffix): (u64, &str) = if value >= 1_000_000_000 {
        (1_000_000_000, "b")
    } else if value >= 1_000_000 {
        (1_000_000, "m")
    } else if value >= 1_000 {
        (1_000, "k")
    } else {
        return value.to_string();
    };
    let tenths = value
        .saturating_mul(10)
        .saturating_add(unit.checked_div(2).unwrap_or(0))
        .checked_div(unit)
        .unwrap_or(0);
    let whole = tenths.checked_div(10).unwrap_or(0);
    if whole >= 1_000 {
        return match suffix {
            "k" => "1m".to_owned(),
            "m" => "1b".to_owned(),
            _ => format!("{whole}{suffix}"),
        };
    }
    let frac = tenths.checked_rem(10).unwrap_or(0);
    if frac == 0 {
        format!("{whole}{suffix}")
    } else {
        format!("{whole}.{frac}{suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::format_diff_count;

    #[test]
    fn counts_match_the_page() {
        assert_eq!(format_diff_count(0), "0");
        assert_eq!(format_diff_count(999), "999");
        assert_eq!(format_diff_count(1_000), "1k");
        assert_eq!(format_diff_count(1_500), "1.5k");
        assert_eq!(format_diff_count(1_000_000), "1m");
        assert_eq!(format_diff_count(2_500_000), "2.5m");
    }
}
