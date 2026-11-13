//! A file's size as the page writes it (`data/workspace/size.ts`): bytes up
//! to a kibibyte, then KB, MB and GB with one decimal under ten (binary
//! units, as the node's bounds are). Worked in whole tenths, as the app's
//! lints keep floating point out; the page's `toFixed` rounds a half up,
//! and so does this.

/// `bytes` in words: "812 B", "3.4 KB", "12 MB".
#[must_use]
pub fn size_word(bytes: u64) -> String {
    const UNITS: [&str; 3] = ["KB", "MB", "GB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut divisor: u128 = 1024;
    let mut unit = 0_usize;
    let whole = u128::from(bytes);
    while whole >= divisor.saturating_mul(1024) && unit.saturating_add(1) < UNITS.len() {
        divisor = divisor.saturating_mul(1024);
        unit = unit.saturating_add(1);
    }
    let name = UNITS.get(unit).copied().unwrap_or("GB");
    // Tenths of the unit, rounded half up.
    let tenths = whole
        .saturating_mul(10)
        .saturating_add(divisor.checked_div(2).unwrap_or(0))
        .checked_div(divisor)
        .unwrap_or(0);
    if whole < divisor.saturating_mul(10) {
        let units = tenths.checked_div(10).unwrap_or(0);
        let tenth = tenths.checked_rem(10).unwrap_or(0);
        format!("{units}.{tenth} {name}")
    } else {
        let rounded = whole
            .saturating_add(divisor.checked_div(2).unwrap_or(0))
            .checked_div(divisor)
            .unwrap_or(0);
        format!("{rounded} {name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_bytes_then_binary_units() {
        assert_eq!(size_word(812), "812 B");
        assert_eq!(size_word(3482), "3.4 KB");
        assert_eq!(size_word(12 * 1024 * 1024), "12 MB");
    }

    #[test]
    fn rounds_as_the_page_does() {
        assert_eq!(size_word(1024), "1.0 KB");
        // Just under ten units keeps its decimal, as `toFixed(1)` does.
        assert_eq!(size_word(10 * 1024 - 30), "10.0 KB");
        assert_eq!(size_word(10 * 1024), "10 KB");
        assert_eq!(size_word(5 * 1024 * 1024 * 1024 * 1024), "5120 GB");
    }
}
