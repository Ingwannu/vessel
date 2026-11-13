//! Setup's output as lines a screen can show while it runs (the worktree
//! start, a worktree's row): the bytes the node pages out
//! (`worktree.output`), decoded, with a terminal's colours and cursor moves
//! taken out and a carriage return's overwrite applied. The page's
//! `data/workspace/setup-log.ts`, ported with its cases.
//!
//! [Decision Log]
//! - Purpose: setup's output is shown as it comes. Setup commands are
//!   `npm ci`, `cargo build`, a script: they write progress bars (`\r`)
//!   and colours (escape sequences) for a terminal.
//! - Constraint: the node keeps 1 MiB per worktree and pages 64 KiB; a
//!   page may end inside a UTF-8 character or an escape sequence.
//! - Alternatives: the terminal view (track C's emulator: right for a
//!   shell, but a whole grid and its parser for a box of text); the raw
//!   text (escape codes on screen, every progress step as a line).
//! - Choice: a streaming UTF-8 decoder, escape sequences dropped by a small
//!   scanner (CSI, OSC, and ESC with its intermediates and final, as
//!   `tput sgr0`'s `ESC ( B`), `\r` overwriting the line as a terminal
//!   would, and erasing the line (`ESC [ K`, `ESC [ 2 K`) applied where the
//!   cursor is; the last [`MAX_LINES`] kept (setup's end is what explains
//!   a failure).
//! - Cost: colours are lost; a program that draws with cursor moves shows
//!   its text out of place. Setup commands are not that.
//!
//! The scanner reads bytes: every byte it acts on is ASCII, and no byte of
//! a multi-byte UTF-8 character is below 0x80, so a position it cuts at is
//! always a character's start. Columns are counted in characters (the page
//! counts UTF-16 units; the two differ only past the Basic Multilingual
//! Plane, where characters are the right count).

use std::collections::VecDeque;

/// How many lines are kept: the end of the output, which is what tells why
/// a setup failed.
pub const MAX_LINES: usize = 2000;

/// Where [`plain`] leaves an erase of the line for `overwrite` to apply:
/// from the cursor to the end (`ESC [ K`, `ESC [ 0 K`), or all of it
/// (`ESC [ 2 K`). Unicode noncharacters, which no program's text holds.
const ERASE_TO_END: char = '\u{FDD0}';
const ERASE_LINE: char = '\u{FDD1}';

const ESC: u8 = 0x1b;
const BEL: u8 = 0x07;

/// The output so far.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupLog {
    /// Finished lines, the oldest dropped past [`MAX_LINES`].
    lines: VecDeque<String>,
    /// The line being written (after the last newline).
    current: String,
    /// Whether lines were dropped from the start.
    cut: bool,
    /// Text held back: an escape sequence cut by the end of a page.
    pending: String,
}

impl SetupLog {
    /// The finished lines.
    pub fn lines(&self) -> impl ExactSizeIterator<Item = &str> {
        self.lines.iter().map(String::as_str)
    }

    /// Whether lines were dropped from the start.
    #[must_use]
    pub const fn cut(&self) -> bool {
        self.cut
    }

    /// An escape sequence held back for the next page.
    #[must_use]
    pub fn pending(&self) -> &str {
        &self.pending
    }

    /// Add `text` (decoded output).
    pub fn append(&mut self, text: &str) {
        let mut held = std::mem::take(&mut self.pending);
        held.push_str(text);
        let (clean, pending) = plain(&held);
        self.pending = pending;
        let mut joined = std::mem::take(&mut self.current);
        joined.push_str(&clean);
        let mut pieces = joined.split('\n').collect::<Vec<_>>();
        pieces
            .pop()
            .unwrap_or_default()
            .clone_into(&mut self.current);
        for piece in pieces {
            self.lines.push_back(overwrite(piece));
            if self.lines.len() > MAX_LINES {
                drop(self.lines.pop_front());
                self.cut = true;
            }
        }
    }

    /// Every line to show, the one being written last (as a terminal shows
    /// it: carriage returns applied).
    #[must_use]
    pub fn shown(&self) -> Vec<String> {
        let current = overwrite(&self.current);
        let mut shown: Vec<String> = self.lines.iter().cloned().collect();
        if !current.is_empty() {
            shown.push(current);
        }
        shown
    }
}

/// `line` after its carriage returns and erases: a terminal starts the line
/// again at each return, writing over what was there, and an erase clears
/// from where the cursor is.
fn overwrite(line: &str) -> String {
    let mut shown: Vec<char> = Vec::new();
    for part in line.split('\r') {
        let mut cursor = 0_usize;
        for character in part.chars() {
            match character {
                ERASE_TO_END => shown.truncate(cursor),
                ERASE_LINE => {
                    shown.clear();
                    shown.resize(cursor, ' ');
                }
                written => {
                    if let Some(slot) = shown.get_mut(cursor) {
                        *slot = written;
                    } else {
                        shown.push(written);
                    }
                    cursor = cursor.saturating_add(1);
                }
            }
        }
    }
    if shown.iter().all(|character| character.is_whitespace()) {
        String::new()
    } else {
        shown.into_iter().collect()
    }
}

/// What an escape sequence leaves in the text: an erase of the line as its
/// marker, anything else nothing.
fn kept(sequence: &str) -> Option<char> {
    match sequence {
        "\u{1b}[K" | "\u{1b}[0K" => Some(ERASE_TO_END),
        "\u{1b}[2K" => Some(ERASE_LINE),
        _other => None,
    }
}

/// Whether `byte` is a control character a line of text has no use for
/// (tab, newline and carriage return are kept).
const fn dropped(byte: u8) -> bool {
    (byte < 0x20 && byte != b'\t' && byte != b'\n' && byte != b'\r') || byte == 0x7f
}

/// Where the escape sequence at `at` ends (the index after it), or `None`
/// when `text` ends before it does.
fn escape_end(text: &str, at: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let kind = *bytes.get(at.checked_add(1)?)?;
    let from = at.checked_add(2)?;
    match kind {
        // CSI: parameters and intermediates, then one final byte.
        b'[' => {
            for (index, &byte) in bytes.iter().enumerate().skip(from) {
                if (0x40..=0x7e).contains(&byte) {
                    return index.checked_add(1);
                }
                if !(0x20..=0x3f).contains(&byte) {
                    return Some(index);
                }
            }
            None
        }
        // OSC: up to BEL, or ESC and a backslash.
        b']' => {
            for (index, &byte) in bytes.iter().enumerate().skip(from) {
                if byte == BEL {
                    return index.checked_add(1);
                }
                if byte == ESC {
                    let after = index.checked_add(2)?;
                    return (after <= bytes.len()).then_some(after);
                }
            }
            None
        }
        // ESC, any intermediates (0x20-0x2F: `(`, `)`, `#`), then one final
        // character: `ESC ( B`, `ESC 7`, `ESC =`.
        _final => {
            for (index, &byte) in bytes.iter().enumerate().skip(at.checked_add(1)?) {
                if !(0x20..=0x2f).contains(&byte) {
                    // A final outside ASCII is taken whole.
                    let width = text
                        .get(index..)
                        .and_then(|rest| rest.chars().next())
                        .map_or(1, char::len_utf8);
                    return index.checked_add(width);
                }
            }
            None
        }
    }
}

/// `text` without escape sequences (an erase of the line left as its
/// marker) and dropped control characters, and what an unfinished sequence
/// at its end held back.
#[must_use]
pub fn plain(text: &str) -> (String, String) {
    let bytes = text.as_bytes();
    let mut clean = String::with_capacity(text.len());
    let mut from = 0_usize;
    let mut index = 0_usize;
    while let Some(&byte) = bytes.get(index) {
        if byte == ESC {
            clean.push_str(text.get(from..index).unwrap_or_default());
            let Some(end) = escape_end(text, index) else {
                return (clean, text.get(index..).unwrap_or_default().to_owned());
            };
            if let Some(marker) = text.get(index..end).and_then(kept) {
                clean.push(marker);
            }
            index = end;
            from = end;
        } else if dropped(byte) {
            clean.push_str(text.get(from..index).unwrap_or_default());
            index = index.saturating_add(1);
            from = index;
        } else {
            index = index.saturating_add(1);
        }
    }
    clean.push_str(text.get(from..).unwrap_or_default());
    (clean, String::new())
}

/// A decoder for one worktree's output: UTF-8 across page ends, a broken
/// sequence read as U+FFFD (as the page's `TextDecoder` reads it).
#[derive(Debug, Clone, Default)]
pub struct OutputDecoder {
    held: Vec<u8>,
}

impl OutputDecoder {
    /// The text of `bytes`, keeping a character cut by the page's end for
    /// the next.
    pub fn text(&mut self, bytes: &[u8]) -> String {
        let mut buffer = std::mem::take(&mut self.held);
        buffer.extend_from_slice(bytes);
        let mut text = String::with_capacity(buffer.len());
        let mut rest = buffer.as_slice();
        loop {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    text.push_str(valid);
                    break;
                }
                Err(error) => {
                    let (valid, after) = rest.split_at(error.valid_up_to());
                    text.push_str(std::str::from_utf8(valid).unwrap_or_default());
                    if let Some(length) = error.error_len() {
                        text.push(char::REPLACEMENT_CHARACTER);
                        rest = after.get(length..).unwrap_or_default();
                    } else {
                        self.held = after.to_vec();
                        break;
                    }
                }
            }
        }
        text
    }
}

#[cfg(test)]
mod tests {
    //! The page's `setup-log.test.ts`, case for case.

    use super::*;

    fn log(text: &str) -> SetupLog {
        let mut log = SetupLog::default();
        log.append(text);
        log
    }

    fn lines(log: &SetupLog) -> Vec<&str> {
        log.lines().collect()
    }

    #[test]
    fn splits_lines_and_keeps_the_one_being_written() {
        let log = log("added 12 packages\nfound 0 vuln");
        assert_eq!(lines(&log), ["added 12 packages"]);
        assert_eq!(log.shown(), ["added 12 packages", "found 0 vuln"]);
    }

    #[test]
    fn drops_colours_and_cursor_moves_keeps_tabs() {
        assert_eq!(
            plain("\u{1b}[1;32mok\u{1b}[0m\tdone\u{1b}[1A").0,
            "ok\tdone"
        );
        assert_eq!(plain("\u{1b}]0;title\u{7}after").0, "after");
        assert_eq!(plain("a\u{8}b\u{7f}").0, "ab");
    }

    #[test]
    fn drops_an_escape_with_intermediates_whole() {
        let bold = log("\u{1b}[1mbold\u{1b}(B\u{1b}[m plain\n");
        assert_eq!(lines(&bold), ["bold plain"]);
        let mut cut = log("a\u{1b}(");
        assert_eq!(cut.pending(), "\u{1b}(");
        cut.append("Bb\n");
        assert_eq!(lines(&cut), ["ab"]);
    }

    #[test]
    fn erases_the_line_from_the_cursor_as_a_progress_bar_does() {
        assert_eq!(lines(&log("Downloading 100%\r\u{1b}[KDone\n")), ["Done"]);
        assert_eq!(lines(&log("Building 3/9\r\u{1b}[2KBuilt\n")), ["Built"]);
        assert_eq!(lines(&log("abcdef\rab\u{1b}[0K\n")), ["ab"]);
        assert!(log("spinner \\\r\u{1b}[2K").shown().is_empty());
    }

    #[test]
    fn holds_back_an_escape_sequence_cut_by_a_page_s_end() {
        let mut log = log("start \u{1b}[3");
        assert_eq!(log.shown(), ["start "]);
        assert_eq!(log.pending(), "\u{1b}[3");
        log.append("2mgreen\u{1b}[0m\n");
        assert_eq!(lines(&log), ["start green"]);
        assert_eq!(log.pending(), "");
    }

    #[test]
    fn writes_over_a_line_at_each_carriage_return() {
        assert_eq!(lines(&log("10%\r55%\r100% done\n")), ["100% done"]);
        assert_eq!(lines(&log("Downloading\rOk\n")), ["Okwnloading"]);
        assert_eq!(lines(&log("one\r\ntwo\r\n")), ["one", "two"]);
    }

    #[test]
    fn keeps_the_last_lines_and_says_it_cut() {
        let many = (0..MAX_LINES.saturating_add(5))
            .map(|index| format!("line {index}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let log = log(&many);
        assert_eq!(log.lines().len(), MAX_LINES);
        assert_eq!(log.lines().next(), Some("line 5"));
        assert!(log.cut());
    }

    #[test]
    fn decodes_a_character_split_across_two_pages() {
        let mut decoder = OutputDecoder::default();
        let bytes = "é".as_bytes();
        assert_eq!(decoder.text(bytes.get(..1).unwrap_or_default()), "");
        assert_eq!(decoder.text(bytes.get(1..).unwrap_or_default()), "é");
    }

    #[test]
    fn reads_a_broken_sequence_as_a_replacement() {
        let mut decoder = OutputDecoder::default();
        assert_eq!(decoder.text(b"a\xffb"), "a\u{fffd}b");
    }

    #[test]
    fn takes_a_final_outside_ascii_whole() {
        assert_eq!(plain("\u{1b}éx").0, "x");
    }
}
