//! Text pasted into a terminal: Paseo's `encodeTerminalPaste`
//! (`packages/app/src/terminal/runtime/terminal-paste.ts`).
//!
//! [Decision Log]
//! - Purpose: a program that turned bracketed paste on (`?2004`, the
//!   grid's `Bracketed`) must see a paste between `ESC [ 200 ~` and
//!   `ESC [ 201 ~`, so a shell does not run each pasted line as it arrives.
//! - Choice: Paseo's rule: without the mode the text goes as typed; with
//!   it, any end marker inside the text loses its ESC first, so a pasted
//!   text cannot end the paste early and run what follows.
//! - Bounds: the result is the text plus twelve bytes; the sender's own
//!   limit (`MAX_PENDING`) applies to it as to keys.

use super::grid::Bracketed;

const START: &str = "\x1b[200~";
const END: &str = "\x1b[201~";
/// The end marker without its ESC: what an end marker inside a paste becomes.
const END_DEFUSED: &str = "[201~";

/// `text` as the program reads a paste of it.
#[must_use]
pub fn encode_paste(text: &str, bracketed: Bracketed) -> String {
    match bracketed {
        Bracketed::Off => text.to_owned(),
        Bracketed::On => format!("{START}{}{END}", text.replace(END, END_DEFUSED)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paste_is_wrapped_only_when_the_program_asked() {
        assert_eq!(encode_paste("ls\n", Bracketed::Off), "ls\n");
        assert_eq!(
            encode_paste("ls\n", Bracketed::On),
            "\x1b[200~ls\n\x1b[201~"
        );
    }

    #[test]
    fn an_end_marker_inside_cannot_end_the_paste() {
        assert_eq!(
            encode_paste("a\x1b[201~rm -rf x\n", Bracketed::On),
            "\x1b[200~a[201~rm -rf x\n\x1b[201~"
        );
    }
}
