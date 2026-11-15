//! Hostile counts, regions and sizes, the two history bounds, and a
//! fixed-seed fuzz over sequences and resizes: nothing panics and every
//! bound holds after each call.

use super::{check, cursor, history, model, play, screen};
use crate::terminal::grid::history::History;
use crate::terminal::grid::{CELL_LIMIT, Cell, Line, Pen};
use crate::terminal::{GridSize, ScrollbackLines, TerminalError};

#[test]
fn enormous_counts_stop_at_the_grid() {
    let mut term = model(10, 4, 10);
    for sequence in [
        &b"\x1b[65535B\x1b[65535C"[..],
        b"\x1b[65535;65535H",
        b"\x1b[65535@\x1b[65535P\x1b[65535X",
        b"\x1b[65535L\x1b[65535M\x1b[65535S\x1b[65535T",
        b"\x1b[65535I\x1b[65535Z\x1b[65535G\x1b[65535d",
        b"\x1b[65535A\x1b[65535D",
        b"\x1b[99999999999999999999;99999999999999999999H",
    ] {
        play(&mut term, sequence);
        check(term.grid());
    }
    assert_eq!(cursor(&term), Some((3, 9)));
}

#[test]
fn a_region_outside_the_grid_or_of_one_row_is_refused() {
    let mut term = model(4, 5, 10);
    play(&mut term, b"\x1b[3;3H");
    for refused in [
        &b"\x1b[900;1000r"[..],
        b"\x1b[5;3r",
        b"\x1b[2;2r",
        b"\x1b[5r",
    ] {
        play(&mut term, refused);
        check(term.grid());
        // Refused: the cursor did not go home.
        assert_eq!(cursor(&term), Some((2, 2)));
    }
    // A bottom past the screen is its last row.
    play(&mut term, b"\x1b[4;900r\x1b[5;1Hx\n");
    check(term.grid());
    assert_eq!(cursor(&term), Some((4, 1)));
    assert!(history(&term).is_empty());
}

#[test]
fn zero_and_enormous_sizes_are_refused() {
    assert_eq!(GridSize::new(0, 24), Err(TerminalError::Empty));
    assert_eq!(GridSize::new(80, 0), Err(TerminalError::Empty));
    assert_eq!(GridSize::new(1025, 24), Err(TerminalError::TooLarge));
    assert_eq!(GridSize::new(80, 513), Err(TerminalError::TooLarge));
    assert_eq!(
        GridSize::new(u16::MAX, u16::MAX),
        Err(TerminalError::TooLarge)
    );
    assert!(GridSize::new(GridSize::MAX_COLS, GridSize::MAX_ROWS).is_ok());
    // A node's size past the bounds is drawn at them; zero is still refused.
    assert_eq!(
        GridSize::within(u16::MAX, u16::MAX),
        GridSize::new(GridSize::MAX_COLS, GridSize::MAX_ROWS)
    );
    assert_eq!(GridSize::within(0, u16::MAX), Err(TerminalError::Empty));
    assert_eq!(
        ScrollbackLines::new(1_000_001),
        Err(TerminalError::Scrollback)
    );
}

#[test]
fn each_refusal_names_the_bound_it_checks() {
    let too_large = TerminalError::TooLarge.to_string();
    assert!(too_large.contains(&GridSize::MAX_COLS.to_string()));
    assert!(too_large.contains(&GridSize::MAX_ROWS.to_string()));
    let scrollback = TerminalError::Scrollback.to_string();
    assert!(scrollback.contains(&ScrollbackLines::MAX.to_string()));
}

#[test]
fn a_one_cell_grid_holds_everything() {
    let mut term = model(1, 1, 3);
    play(
        &mut term,
        "a한b\r\n\t\x1b[2;2r\x1b[?6h\x1b[5;5H\x1b[L\x1b[M\x1bM\x1bD\x1bE".as_bytes(),
    );
    check(term.grid());
    assert_eq!(cursor(&term), Some((0, 0)));
    assert!(history(&term).len() <= 3);
}

#[test]
fn history_keeps_at_most_the_settings_lines() {
    let mut term = model(4, 2, 3);
    for line in 0..20_u8 {
        play(&mut term, &[b'a'.saturating_add(line), b'\r', b'\n']);
    }
    assert_eq!(history(&term), ["q", "r", "s"]);
    assert_eq!(screen(&term), ["t", ""]);
}

#[test]
fn a_cell_is_twenty_bytes_so_history_is_at_most_eighty_mib() {
    assert_eq!(size_of::<Cell>(), 20);
    assert_eq!(
        CELL_LIMIT.saturating_mul(size_of::<Cell>()),
        80 * 1024 * 1024
    );
}

#[test]
fn history_keeps_at_most_its_cells_whatever_the_setting() {
    let mut history = History::new(ScrollbackLines::new(ScrollbackLines::MAX).unwrap_or_default());
    let wide = usize::from(GridSize::MAX_COLS);
    let lines = CELL_LIMIT
        .checked_div(wide)
        .unwrap_or(0)
        .saturating_add(100);
    for _ in 0..lines {
        let mut line = Line::blank(GridSize::MAX_COLS, Cell::BLANK);
        line.cells_mut().fill(Cell::printed('x', Pen::PLAIN));
        history.push(line);
    }
    assert!(history.cells() <= CELL_LIMIT);
    assert_eq!(history.len(), CELL_LIMIT.checked_div(wide).unwrap_or(0));
    // Blank lines take no cells, so the line bound is the one that holds.
    history.clear();
    for _ in 0..2_000 {
        history.push(Line::blank(GridSize::MAX_COLS, Cell::BLANK));
    }
    assert_eq!(history.cells(), 0);
    assert_eq!(history.len(), 2_000);
}

/// A fixed-seed generator (xorshift64), so a failure is repeatable.
struct Seeded(u64);

impl Seeded {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0.wrapping_shl(13);
        self.0 ^= self.0.wrapping_shr(7);
        self.0 ^= self.0.wrapping_shl(17);
        self.0
    }

    /// A value below `bound` (`bound` at least 1).
    fn below(&mut self, bound: u64) -> u64 {
        self.next().checked_rem(bound).unwrap_or(0)
    }

    fn pick<'a>(&mut self, from: &'a [&'a [u8]]) -> &'a [u8] {
        let len = u64::try_from(from.len()).unwrap_or(1);
        let index = usize::try_from(self.below(len)).unwrap_or(0);
        from.get(index).copied().unwrap_or_default()
    }

    /// A number as text: small, at the edges, or past `u16`.
    fn number(&mut self, out: &mut Vec<u8>) {
        let value = match self.below(4) {
            0 => self.below(10),
            1 => self.below(600),
            2 => 65_535,
            _ => self.next(),
        };
        out.extend_from_slice(value.to_string().as_bytes());
    }

    /// One token: text, a control, a CSI with odd parameters, a mode, an
    /// escape, an SGR with sub-parameters, or a raw byte.
    fn token(&mut self, out: &mut Vec<u8>) {
        const FINALS: &[u8] = b"ABCDEFGHIJKLMPSTXZ@`abdefghlmnrsu";
        const MODES: &[&[u8]] = &[
            b"1", b"4", b"6", b"7", b"20", b"25", b"47", b"1047", b"1048", b"1049", b"2004",
        ];
        const ESCAPES: &[&[u8]] = &[b"D", b"E", b"H", b"M", b"7", b"8", b"c", b"(B", b"#8"];
        const TEXT: &[&[u8]] = &[
            b"text ",
            "한글 ".as_bytes(),
            "e\u{301}".as_bytes(),
            "\u{1112}\u{1161}\u{11ab}".as_bytes(),
            "\u{1f600}\u{200d}".as_bytes(),
            "\u{0e34}漢".as_bytes(),
        ];
        const CONTROLS: &[&[u8]] = &[b"\r", b"\n", b"\t", b"\x08", b"\x0b", b"\x07", b"\r\n"];
        match self.below(9) {
            0 | 1 => out.extend_from_slice(self.pick(TEXT)),
            2 => out.extend_from_slice(self.pick(CONTROLS)),
            3 | 4 => {
                out.extend_from_slice(b"\x1b[");
                for _ in 0..self.below(4) {
                    self.number(out);
                    out.push(b';');
                }
                self.number(out);
                let index = usize::try_from(self.below(33)).unwrap_or(0);
                out.push(FINALS.get(index).copied().unwrap_or(b'H'));
            }
            5 => {
                out.extend_from_slice(b"\x1b[?");
                out.extend_from_slice(self.pick(MODES));
                out.push(if self.below(2) == 0 { b'h' } else { b'l' });
            }
            6 => {
                out.push(0x1b);
                out.extend_from_slice(self.pick(ESCAPES));
            }
            7 => {
                // SGR with colon sub-parameters, often cut short.
                out.extend_from_slice(b"\x1b[");
                out.extend_from_slice(self.pick(&[&b"38"[..], b"48", b"58", b"4"]));
                for _ in 0..self.below(7) {
                    out.push(b':');
                    self.number(out);
                }
                out.push(b'm');
            }
            _ => out.push(self.next().to_le_bytes().first().copied().unwrap_or(0)),
        }
    }

    /// A size, now and then the smallest or the largest.
    fn size(&mut self) -> GridSize {
        let (cols, rows) = match self.below(10) {
            0 => (1, 1),
            1 => (GridSize::MAX_COLS, GridSize::MAX_ROWS),
            _ => (
                u16::try_from(self.below(300))
                    .unwrap_or(0)
                    .saturating_add(1),
                u16::try_from(self.below(80)).unwrap_or(0).saturating_add(1),
            ),
        };
        GridSize::new(cols, rows).unwrap_or_default()
    }
}

#[test]
fn random_sequences_and_resizes_never_panic_and_keep_every_bound() {
    let mut seeded = Seeded(0x9e37_79b9_7f4a_7c15);
    let mut term = model(80, 24, 200);
    let mut written = 0_usize;
    while written < 2 * 1024 * 1024 {
        let mut bytes = Vec::new();
        while bytes.len() < 4096 {
            seeded.token(&mut bytes);
        }
        written = written.saturating_add(bytes.len());
        play(&mut term, &bytes);
        check(term.grid());
        if seeded.below(16) == 0 {
            term.resize(seeded.size());
            check(term.grid());
        }
    }
}
