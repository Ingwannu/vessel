//! Clipboard preparation follows Paseo's paste codec, before the ordered sender.
//!
//! One complete paste is admitted or refused before sending: truncating a marker
//! would leave the program inside paste mode. The existing node byte bound is the
//! authority, so multi-byte text cannot bypass it through the sender's char count.
use super::{TerminalError, grid::Bracketed};
use vessel_proto::terminals::SendTerminal;

const START: &str = "\x1b[200~";
const END: &str = "\x1b[201~";
const DEFUSED_END: &str = "[201~";

/// Prepare nonempty clipboard text as one bounded paste, without sending it.
/// Bracketed mode defuses embedded end markers exactly as Paseo does.
///
/// # Errors
/// Returns [`TerminalError::InputTooLarge`] before allocating an oversized packet.
pub fn prepare_paste(text: &str, mode: Bracketed) -> Result<Option<String>, TerminalError> {
    if text.is_empty() {
        return Ok(None);
    }
    let overhead = match mode {
        Bracketed::Off => 0,
        Bracketed::On => START.len().saturating_add(END.len()),
    };
    let defused = match mode {
        Bracketed::Off => 0,
        Bracketed::On => text.matches(END).count(),
    };
    // Each defused marker removes its one-byte ESC; checked addition refuses
    // overflow rather than allocating or partially queuing an incomplete paste.
    let bytes = text.len().saturating_sub(defused).checked_add(overhead);
    if bytes.is_none_or(|bytes| bytes > SendTerminal::MAX_BYTES) {
        return Err(TerminalError::InputTooLarge);
    }
    Ok(Some(match mode {
        Bracketed::Off => text.to_owned(),
        Bracketed::On => format!("{START}{}{END}", text.replace(END, DEFUSED_END)),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paste_keeps_unicode_newlines_and_defuses_every_embedded_end_marker() {
        let text = "한글\r\nfirst\n\x1b[201~second\x1b[201~\x1b[200~";
        assert_eq!(prepare_paste(text, Bracketed::Off).unwrap().unwrap(), text);
        assert_eq!(
            prepare_paste(text, Bracketed::On).unwrap().unwrap(),
            "\x1b[200~한글\r\nfirst\n[201~second[201~\x1b[200~\x1b[201~"
        );
        assert_eq!(prepare_paste("", Bracketed::On).unwrap(), None);
        assert_eq!(prepare_paste("", Bracketed::Off).unwrap(), None);
    }

    #[test]
    fn complete_packet_uses_the_node_byte_limit_including_markers() {
        let exact = "x".repeat(SendTerminal::MAX_BYTES);
        assert_eq!(
            prepare_paste(&exact, Bracketed::Off)
                .unwrap()
                .unwrap()
                .len(),
            SendTerminal::MAX_BYTES
        );
        assert_eq!(
            prepare_paste(&exact, Bracketed::On),
            Err(TerminalError::InputTooLarge)
        );
        let room = SendTerminal::MAX_BYTES
            .checked_sub(START.len() + END.len())
            .unwrap();
        assert_eq!(
            prepare_paste(&"x".repeat(room), Bracketed::On)
                .unwrap()
                .unwrap()
                .len(),
            SendTerminal::MAX_BYTES
        );
        let wide = "한".repeat(
            SendTerminal::MAX_BYTES
                .checked_div(3)
                .unwrap()
                .saturating_add(1),
        );
        assert!(wide.chars().count() < SendTerminal::MAX_BYTES);
        assert_eq!(
            prepare_paste(&wide, Bracketed::Off),
            Err(TerminalError::InputTooLarge)
        );
    }
}
