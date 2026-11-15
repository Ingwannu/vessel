//! Windows prefix decoding on every host; no native Path iterator may normalize input.

use super::{InvalidExecPath, Separators, valid_part, validate_parts};

pub(super) fn validate(text: &str) -> Result<(), InvalidExecPath> {
    if let Some(tail) = text.strip_prefix(r"\\?\") {
        return verbatim(tail);
    }
    if let Some(tail) = network_root(text) {
        if let Some(device) = tail
            .strip_prefix('.')
            .and_then(|tail| separator(tail, Separators::Windows))
        {
            let body = after_root_component(device, Separators::Windows)?;
            return validate_parts(body, Separators::Windows);
        }
        let share = after_root_component(tail, Separators::Windows)?;
        let body = after_root_component(share, Separators::Windows)?;
        return validate_parts(body, Separators::Windows);
    }
    let body = drive_root(text, Separators::Windows).ok_or(InvalidExecPath::NotAbsolute)?;
    validate_parts(body, Separators::Windows)
}

fn verbatim(text: &str) -> Result<(), InvalidExecPath> {
    // Rust recognizes either separator in the UNC marker before applying
    // verbatim component rules. Treating it as a generic namespace could
    // mistake the share itself for an executable.
    if let Some(tail) = text
        .strip_prefix("UNC")
        .and_then(|tail| separator(tail, Separators::Windows))
    {
        let share = after_root_component(tail, Separators::Verbatim)?;
        let body = after_root_component(share, Separators::Verbatim)?;
        return validate_parts(body, Separators::Verbatim);
    }
    if let Some(body) = drive_root(text, Separators::Windows) {
        // A forward slash after a verbatim drive is not its root separator.
        if drive_root(text, Separators::Verbatim).is_none() {
            return Err(InvalidExecPath::NotAbsolute);
        }
        return validate_parts(body, Separators::Verbatim);
    }
    let body = after_root_component(text, Separators::Verbatim)?;
    validate_parts(body, Separators::Verbatim)
}

fn network_root(text: &str) -> Option<&str> {
    separator(text, Separators::Windows).and_then(|tail| separator(tail, Separators::Windows))
}

fn separator(text: &str, separators: Separators) -> Option<&str> {
    let mut chars = text.chars();
    separators.matches(chars.next()?).then_some(chars.as_str())
}

fn drive_root(text: &str, separators: Separators) -> Option<&str> {
    let mut chars = text.chars();
    if !chars.next()?.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }
    separators.matches(chars.next()?).then_some(chars.as_str())
}

fn after_root_component(text: &str, separators: Separators) -> Result<&str, InvalidExecPath> {
    let (prefix, body) = text
        .split_once(|c| separators.matches(c))
        .ok_or(InvalidExecPath::NoProgram)?;
    if !valid_part(prefix) {
        return Err(InvalidExecPath::Component);
    }
    Ok(body)
}
