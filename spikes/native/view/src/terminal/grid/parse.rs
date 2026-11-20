//! `vte` is the decoding edge. Past it the grid holds enums, and a query
//! is reply bytes rather than a cell. The bytes are Paseo's
//! (`terminal.ts` 45–49 and 883–922).

use vte::{Params, Perform};

use super::super::keys::CursorKeys;
use super::super::paste::Bracketed;
use super::super::types::{Attrs, Colour, Polarity, Rule, Slant, Visibility, Weight};
use super::TerminalModel;

pub(super) struct Step<'a> {
    pub(super) model: &'a mut TerminalModel,
    pub(super) replies: Vec<u8>,
}

enum Marker {
    Plain,
    Private,
    Other,
}

#[derive(Clone, Copy)]
enum Enable {
    On,
    Off,
}

enum CursorForm {
    Plain,
    Private,
}

impl Perform for Step<'_> {
    fn print(&mut self, ch: char) {
        self.model.print_char(ch);
    }

    fn execute(&mut self, byte: u8) {
        self.model.execute_control(byte);
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let (Some(code), Some(data)) = (params.first().copied(), params.get(1).copied()) else {
            return;
        };
        if data != b"?" {
            return;
        }
        let reply: &[u8] = match code {
            b"10" => b"\x1b]10;rgb:e6e6/e6e6/e6e6\x1b\\",
            b"11" => b"\x1b]11;rgb:0b0b/0b0b/0b0b\x1b\\",
            b"12" => b"\x1b]12;rgb:e6e6/e6e6/e6e6\x1b\\",
            _ => return,
        };
        self.replies.extend_from_slice(reply);
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }
        match marker(intermediates) {
            Marker::Other => {}
            Marker::Private => self.private_csi(params, action),
            Marker::Plain => self.plain_csi(params, action),
        }
    }
}

impl Step<'_> {
    fn plain_csi(&mut self, params: &Params, action: char) {
        match action {
            'H' | 'f' => self.cup(params),
            'A' => self.model.move_by(neg(steps(params)), 0),
            'B' => self.model.move_by(steps(params), 0),
            'C' => self.model.move_by(0, steps(params)),
            'D' => self.model.move_by(0, neg(steps(params))),
            'G' => self.column(params),
            'd' => self.row(params),
            'J' => self.model.erase_display(first_or_zero(params)),
            'K' => self.model.erase_line(first_or_zero(params)),
            'm' => apply_sgr(self.model, params),
            'c' if primary_da(params) => self.replies.extend_from_slice(b"\x1b[?62;4;22c"),
            'n' => self.status(params, CursorForm::Plain),
            _ => {}
        }
    }

    fn private_csi(&mut self, params: &Params, action: char) {
        match action {
            'h' => modes(self.model, params, Enable::On),
            'l' => modes(self.model, params, Enable::Off),
            'n' => self.status(params, CursorForm::Private),
            _ => {}
        }
    }

    fn cup(&mut self, params: &Params) {
        let (row, col) = pair(params);
        self.model
            .move_cursor(row.saturating_sub(1), col.saturating_sub(1));
    }

    fn column(&mut self, params: &Params) {
        let col = one_based(first_or_zero(params));
        self.model
            .move_cursor(self.model.cursor_pos().0, col.saturating_sub(1));
    }

    fn row(&mut self, params: &Params) {
        let row = one_based(first_or_zero(params));
        self.model
            .move_cursor(row.saturating_sub(1), self.model.cursor_pos().1);
    }

    fn status(&mut self, params: &Params, form: CursorForm) {
        let Some(code) = only(params) else {
            return;
        };
        if let CursorForm::Plain = form {
            if code == 5 {
                self.replies.extend_from_slice(b"\x1b[0n");
                return;
            }
        }
        if code == 6 {
            let (row, col) = self.model.cursor_pos();
            push_cursor(
                &mut self.replies,
                form,
                row.saturating_add(1),
                col.saturating_add(1),
            );
        }
    }
}

fn marker(intermediates: &[u8]) -> Marker {
    match intermediates {
        [] => Marker::Plain,
        [b'?'] => Marker::Private,
        _ => Marker::Other,
    }
}

fn modes(model: &mut TerminalModel, params: &Params, enable: Enable) {
    for group in params.iter() {
        let Some(code) = group.first().copied() else {
            continue;
        };
        apply_mode(model, code, enable);
    }
}

fn apply_mode(model: &mut TerminalModel, code: u16, enable: Enable) {
    match code {
        1 => match enable {
            Enable::On => model.set_cursor_keys(CursorKeys::Application),
            Enable::Off => model.set_cursor_keys(CursorKeys::Normal),
        },
        25 => match enable {
            Enable::On => model.show_cursor(Visibility::Shown),
            Enable::Off => model.show_cursor(Visibility::Hidden),
        },
        47 | 1047 | 1049 => match enable {
            Enable::On => model.enter_alternate(),
            Enable::Off => model.leave_alternate(),
        },
        2004 => match enable {
            Enable::On => model.set_bracketed(Bracketed::On),
            Enable::Off => model.set_bracketed(Bracketed::Off),
        },
        _ => {}
    }
}

fn apply_sgr(model: &mut TerminalModel, params: &Params) {
    let codes = flat(params);
    if codes.is_empty() {
        model.set_attrs(Attrs::plain());
        return;
    }
    let mut attrs = model.attrs();
    let mut index = 0usize;
    while index < codes.len() {
        let code = codes.get(index).copied().unwrap_or(0);
        index = index.saturating_add(1);
        match code {
            0 => attrs = Attrs::plain(),
            1 => attrs.weight = Weight::Bold,
            3 => attrs.slant = Slant::Italic,
            4 => attrs.rule = Rule::Single,
            7 => attrs.polarity = Polarity::Inverse,
            22 => attrs.weight = Weight::Regular,
            23 => attrs.slant = Slant::Roman,
            24 => attrs.rule = Rule::None,
            27 => attrs.polarity = Polarity::Normal,
            39 => attrs.fg = Colour::Default,
            49 => attrs.bg = Colour::Default,
            30..=37 => attrs.fg = Colour::Indexed(shift(code, 30, 0)),
            40..=47 => attrs.bg = Colour::Indexed(shift(code, 40, 0)),
            90..=97 => attrs.fg = Colour::Indexed(shift(code, 90, 8)),
            100..=107 => attrs.bg = Colour::Indexed(shift(code, 100, 8)),
            38 | 48 => {
                let colour = take_colour(&codes, &mut index);
                if code == 38 {
                    attrs.fg = colour;
                } else {
                    attrs.bg = colour;
                }
            }
            _ => {}
        }
    }
    model.set_attrs(attrs);
}

fn take_colour(codes: &[u16], index: &mut usize) -> Colour {
    let kind = codes.get(*index).copied().unwrap_or(0);
    *index = index.saturating_add(1);
    if kind == 5 {
        let value = codes.get(*index).copied().unwrap_or(0);
        *index = index.saturating_add(1);
        return u8::try_from(value)
            .map(Colour::Indexed)
            .unwrap_or(Colour::Default);
    }
    if kind == 2 {
        let red = byte_at(codes, index);
        let green = byte_at(codes, index);
        let blue = byte_at(codes, index);
        return Colour::Rgb(red, green, blue);
    }
    Colour::Default
}

fn byte_at(codes: &[u16], index: &mut usize) -> u8 {
    let value = codes.get(*index).copied().unwrap_or(0);
    *index = index.saturating_add(1);
    u8::try_from(value).unwrap_or(0)
}

fn shift(code: u16, base: u16, extra: u16) -> u8 {
    let sum = code.saturating_sub(base).saturating_add(extra);
    u8::try_from(sum).unwrap_or(0)
}

fn flat(params: &Params) -> Vec<u16> {
    let mut out = Vec::new();
    for group in params.iter() {
        if group.is_empty() {
            out.push(0);
            continue;
        }
        for value in group {
            out.push(*value);
        }
    }
    out
}

fn first_or_zero(params: &Params) -> u16 {
    params
        .iter()
        .next()
        .and_then(|group| group.first().copied())
        .unwrap_or(0)
}

fn one_based(raw: u16) -> u16 {
    if raw == 0 { 1 } else { raw }
}

fn steps(params: &Params) -> i16 {
    let count = one_based(first_or_zero(params));
    i16::try_from(count).unwrap_or(i16::MAX)
}

fn neg(steps: i16) -> i16 {
    steps.saturating_neg()
}

fn pair(params: &Params) -> (u16, u16) {
    let mut groups = params.iter();
    let row = one_based(
        groups
            .next()
            .and_then(|group| group.first().copied())
            .unwrap_or(0),
    );
    let col = one_based(
        groups
            .next()
            .and_then(|group| group.first().copied())
            .unwrap_or(0),
    );
    (row, col)
}

fn only(params: &Params) -> Option<u16> {
    let mut groups = params.iter();
    let value = groups.next().and_then(|group| group.first().copied())?;
    if groups.next().is_some() {
        return None;
    }
    Some(value)
}

fn primary_da(params: &Params) -> bool {
    if params.is_empty() {
        return true;
    }
    match only(params) {
        Some(0) => true,
        Some(_) | None => false,
    }
}

fn push_cursor(out: &mut Vec<u8>, form: CursorForm, row: u16, col: u16) {
    out.extend_from_slice(b"\x1b[");
    if let CursorForm::Private = form {
        out.push(b'?');
    }
    push_u16(out, row);
    out.push(b';');
    push_u16(out, col);
    out.push(b'R');
}

/// Decimal without division. Each scale is subtracted off, so a digit
/// never needs `/` or `%`.
fn push_u16(out: &mut Vec<u8>, mut value: u16) {
    let mut started = false;
    for scale in [10_000_u16, 1_000, 100, 10, 1] {
        let mut digit = 0u16;
        while value >= scale {
            value = value.saturating_sub(scale);
            digit = digit.saturating_add(1);
        }
        if digit == 0 && !started && scale != 1 {
            continue;
        }
        started = true;
        let byte = u8::try_from(digit).unwrap_or(0).saturating_add(b'0');
        out.push(byte);
    }
}
