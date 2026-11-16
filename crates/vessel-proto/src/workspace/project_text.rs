//! JavaScript whitespace at project-file input edges, before typed validation.
//! No command, path or digest is constructed here.

/// Keep project-file text consistent with Paseo without changing interior text.
/// JavaScript trims BOM and line/paragraph separators, but retains NEL.
#[must_use]
pub fn trim_project_text(text: &str) -> &str {
    crate::trim_ecmascript(text)
}

#[cfg(test)]
mod tests {
    use super::trim_project_text;

    #[test]
    fn project_edges_follow_javascript_without_trimming_interior_text() {
        for edge in [" \t\r\n", "\u{feff}", "\u{2028}", "\u{2029}"] {
            let input = format!("{edge}program{edge}");
            assert_eq!(trim_project_text(&input), "program");
        }
        let nel = "\u{0085}program\u{0085}";
        assert_eq!(trim_project_text(nel), nel);
        assert_eq!(
            trim_project_text("\u{feff}program\u{2028}argument\u{2029}"),
            "program\u{2028}argument"
        );
    }
}
