//! Location suffixes are stripped before decoding a path so encoded colons stay literal.

use super::Lines;
use crate::matcher::whitespace;

pub(super) fn extract(value: &str) -> Option<(&str, Option<Lines>)> {
    if let Some((path, fragment)) = value.rsplit_once('#') {
        if !fragment.starts_with(['L', 'l']) {
            return None;
        }
        let (start, end) = fragment
            .split_once('-')
            .map_or((fragment, None), |(start, end)| (start, Some(end)));
        return Some((
            path,
            Some(Lines::parse(
                fragment_line(start)?,
                match end {
                    Some(end) => Some(fragment_line(end)?),
                    None => None,
                },
            )?),
        ));
    }
    if let Some((path, location)) = value
        .strip_suffix(')')
        .and_then(|value| value.rsplit_once('('))
    {
        return Some((
            path.trim_end_matches(whitespace),
            Some(parse_range(location, ',')?),
        ));
    }
    for marker in [" lines ", " line "] {
        if let Some((path, location)) = value.rsplit_once(marker) {
            return Some((
                path.trim_end_matches(whitespace),
                Some(parse_range(location, ':')?),
            ));
        }
    }
    // The first suffix that consists solely of valid line/column syntax wins;
    // a Windows drive colon is skipped because its remainder is not numeric.
    for (index, _) in value.match_indices(':') {
        let after = value.get(index.saturating_add(1)..)?;
        if let Some(lines) = parse_range(after, ':') {
            return Some((value.get(..index)?, Some(lines)));
        }
    }
    Some((value, None))
}

fn parse_range(value: &str, column_separator: char) -> Option<Lines> {
    let (start, end) = value
        .split_once('-')
        .map_or((value, None), |(a, b)| (a, Some(b)));

    Lines::parse(
        line_without_column(start, column_separator)?,
        match end {
            Some(end) => Some(line_without_column(end, column_separator)?),
            None => None,
        },
    )
}

fn line_without_column(value: &str, separator: char) -> Option<&str> {
    let mut parts = value.split(separator);
    let line = parts.next()?;
    if let Some(column) = parts.next()
        && !super::digits(column)
    {
        return None;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(line)
}

fn fragment_line(value: &str) -> Option<&str> {
    let value = value
        .strip_prefix('L')
        .or_else(|| value.strip_prefix('l'))
        .unwrap_or(value);
    let mut parts = value.split(['C', 'c']);
    let line = parts.next()?;
    if let Some(column) = parts.next()
        && !super::digits(column)
    {
        return None;
    }
    if parts.next().is_some() || !super::digits(line) {
        return None;
    }
    Some(line)
}
