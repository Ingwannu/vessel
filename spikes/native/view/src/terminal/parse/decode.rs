//! `vte`'s dispatches to typed actions: which final byte, which private
//! marker, which parameters, as the enums of `action`. A sequence not
//! listed is `Unsupported` (dropped); a query not in ADR 0073's closed
//! set is `RefusedQuery` (never answered).

use vte::Params;

use super::action::{
    Attributes, ColourSlot, Count, Csi, Dcs, Erase, Esc, Group, Mode, Osc, Status, Switch, TabClear,
};

/// The CSI `final` with `intermediates` (private markers `?`, `>`, `=`
/// and `<` are among them, as `vte` collects them) and `params`.
/// `ignore`: the parser dropped parameters or intermediates past its
/// limits, so the sequence is not trusted.
pub(super) fn csi(params: &Params, intermediates: &[u8], ignore: bool, action: char) -> Csi {
    if ignore {
        return Csi::Unsupported;
    }
    match intermediates {
        [] => plain(params, action),
        [b'?'] => private(params, action),
        [b'>' | b'='] if action == 'c' => Csi::Attributes(Attributes::Other),
        [b' '] if action == 'q' => Csi::CursorStyle,
        // DECRQM, ANSI and private forms.
        [b'$'] | [b'?', b'$'] if action == 'p' => Csi::RefusedQuery,
        _ => Csi::Unsupported,
    }
}

fn plain(params: &Params, action: char) -> Csi {
    let count = || Count::of(first(params));
    match action {
        'A' => Csi::Up(count()),
        'B' | 'e' => Csi::Down(count()),
        'C' | 'a' => Csi::Forward(count()),
        'D' => Csi::Back(count()),
        'E' => Csi::NextLine(count()),
        'F' => Csi::PreviousLine(count()),
        'G' | '`' => Csi::Column(count()),
        'H' | 'f' => Csi::Goto(Count::of(nth(params, 0)), Count::of(nth(params, 1))),
        'd' => Csi::Row(count()),
        'J' => erase(first(params), Csi::EraseDisplay),
        'K' => match first(params) {
            3 => Csi::Unsupported,
            code => erase(code, Csi::EraseLine),
        },
        'L' => Csi::InsertLines(count()),
        'M' => Csi::DeleteLines(count()),
        '@' => Csi::InsertChars(count()),
        'P' => Csi::DeleteChars(count()),
        'X' => Csi::EraseChars(count()),
        'S' => Csi::ScrollUp(count()),
        'T' => Csi::ScrollDown(count()),
        'I' => Csi::TabForward(count()),
        'Z' => Csi::TabBack(count()),
        'g' => match first(params) {
            0 => Csi::ClearTabs(TabClear::Here),
            3 => Csi::ClearTabs(TabClear::All),
            _ => Csi::Unsupported,
        },
        'r' => Csi::ScrollRegion(optional(nth(params, 0)), optional(nth(params, 1))),
        'h' => Csi::Modes(Switch::Set, modes(params, ansi_mode)),
        'l' => Csi::Modes(Switch::Reset, modes(params, ansi_mode)),
        'm' => Csi::Graphic(groups(params)),
        // Exactly one parameter, as Paseo's daemon asks (`terminal.ts`
        // 892: `params.length !== 1` is not answered).
        'n' => match (params.len(), first(params)) {
            (1, 5) => Csi::Status(Status::Operating),
            (1, 6) => Csi::Status(Status::Cursor),
            _ => Csi::RefusedQuery,
        },
        // Primary device attributes: no parameter, or 0, and nothing else.
        'c' => {
            if bare(params) {
                Csi::Attributes(Attributes::Primary)
            } else {
                Csi::Attributes(Attributes::Other)
            }
        }
        // With parameters `s` is DECSLRM (margins), not kept.
        's' if bare(params) => Csi::SaveCursor,
        'u' if bare(params) => Csi::RestoreCursor,
        // Window operations, the title and size reports among them.
        't' => Csi::RefusedQuery,
        _ => Csi::Unsupported,
    }
}

fn private(params: &Params, action: char) -> Csi {
    match action {
        'h' => Csi::Modes(Switch::Set, modes(params, private_mode)),
        'l' => Csi::Modes(Switch::Reset, modes(params, private_mode)),
        // DECSED and DECSEL erase as ED and EL do (protection is not kept).
        'J' => erase(first(params), Csi::EraseDisplay),
        'K' => match first(params) {
            3 => Csi::Unsupported,
            code => erase(code, Csi::EraseLine),
        },
        'n' => match (params.len(), first(params)) {
            (1, 6) => Csi::Status(Status::PrivateCursor),
            _ => Csi::RefusedQuery,
        },
        // The kitty keyboard query: never answered. The input slice
        // encodes keys as xterm does, never with kitty's flags, so an answer
        // (even `CSI ? 0 u`) would tell a program the protocol is there.
        // Its push, pop and set (`CSI > u`, `CSI < u`, `CSI = u`) carry
        // other markers and are unsupported.
        'u' => Csi::RefusedQuery,
        _ => Csi::Unsupported,
    }
}

fn erase(code: u16, make: fn(Erase) -> Csi) -> Csi {
    match code {
        0 => make(Erase::ToEnd),
        1 => make(Erase::ToStart),
        2 => make(Erase::All),
        3 => make(Erase::Scrollback),
        _ => Csi::Unsupported,
    }
}

const fn ansi_mode(code: u16) -> Mode {
    match code {
        4 => Mode::Insert,
        20 => Mode::NewLine,
        _ => Mode::Other,
    }
}

const fn private_mode(code: u16) -> Mode {
    match code {
        1 => Mode::CursorKeys,
        6 => Mode::Origin,
        7 => Mode::AutoWrap,
        25 => Mode::ShowCursor,
        47 | 1047 => Mode::AltScreen,
        1048 => Mode::SaveCursor,
        1049 => Mode::AltScreenSaveCursor,
        2004 => Mode::BracketedPaste,
        _ => Mode::Other,
    }
}

fn modes(params: &Params, kind: fn(u16) -> Mode) -> Vec<Mode> {
    params
        .iter()
        .map(|group| kind(group.first().copied().unwrap_or(0)))
        .collect()
}

/// SGR's groups as sent; an empty `CSI m` is one group `[0]` (reset).
/// `vte` already sends `[0]` for it; the guard keeps that true if it ever
/// sends an empty list.
fn groups(params: &Params) -> Vec<Group> {
    if params.is_empty() {
        return vec![vec![0]];
    }
    params.iter().map(<[u16]>::to_vec).collect()
}

/// Whether the sequence carried no parameter. `vte` dispatches the
/// parameter being collected even when none was written, so `CSI s`
/// arrives as one parameter of 0 (never as an empty list), the same as
/// `CSI 0 s`.
fn bare(params: &Params) -> bool {
    params.len() <= 1 && first(params) == 0
}

/// The first parameter's value; 0 when there is none.
fn first(params: &Params) -> u16 {
    nth(params, 0)
}

/// Parameter `index`'s first value; 0 when there is none.
fn nth(params: &Params, index: usize) -> u16 {
    params
        .iter()
        .nth(index)
        .and_then(|group| group.first().copied())
        .unwrap_or(0)
}

/// A position that may be absent: 0 means absent.
fn optional(raw: u16) -> Option<Count> {
    (raw != 0).then(|| Count::of(raw))
}

/// An escape sequence's `byte` after `intermediates`.
pub(super) const fn esc(intermediates: &[u8], ignore: bool, byte: u8) -> Esc {
    if ignore || !intermediates.is_empty() {
        // Charset designations (`ESC ( B`), DECALN (`ESC # 8`) and the rest.
        return Esc::Other;
    }
    match byte {
        b'D' => Esc::Index,
        b'E' => Esc::NextLine,
        b'H' => Esc::SetTab,
        b'M' => Esc::ReverseIndex,
        b'7' => Esc::SaveCursor,
        b'8' => Esc::RestoreCursor,
        b'c' => Esc::Reset,
        _ => Esc::Other,
    }
}

/// An OSC's parameters (`vte` splits them at `;` and keeps at most
/// `OSC_LIMIT` bytes of the whole string).
pub(super) fn osc(params: &[&[u8]]) -> Osc {
    let code = params.first().copied().unwrap_or_default();
    let rest = params.get(1..).unwrap_or_default();
    match code {
        // A title may hold `;`: what the parser split is joined again.
        b"0" | b"2" => Osc::Title(String::from_utf8_lossy(&rest.join(&b';')).into_owned()),
        b"10" | b"11" | b"12" => match rest {
            [b"?"] => Osc::ColourQuery(match code {
                b"10" => ColourSlot::Foreground,
                b"11" => ColourSlot::Background,
                _ => ColourSlot::Cursor,
            }),
            _ => Osc::Other,
        },
        // `OSC 52 ; selection ; ?` reads the clipboard.
        b"52" => match rest {
            [_, b"?"] => Osc::ClipboardRead,
            _ => Osc::Other,
        },
        _ => Osc::Other,
    }
}

/// What a DCS asks, from its hook.
pub(super) fn dcs(intermediates: &[u8], ignore: bool, action: char) -> Dcs {
    match (ignore, intermediates, action) {
        (false, [b'$'], 'q') => Dcs::RequestSetting,
        (false, [b'+'], 'q') => Dcs::TermcapQuery,
        _ => Dcs::Other,
    }
}
