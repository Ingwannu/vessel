//! Parse external key names once; encoding only sees the closed key set.
use super::InputError;

/// One physical named key or a single BMP key value; committed text is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key(Kind);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Character(char),
    Enter,
    Tab,
    Backspace,
    Escape,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    Insert,
    Delete,
    PageUp,
    PageDown,
    Function(Function),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Function {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Eleven,
    Twelve,
}

impl Key {
    pub(super) const fn kind(self) -> Kind {
        self.0
    }
}

impl TryFrom<&str> for Key {
    type Error = InputError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        let kind = match text {
            "Enter" => Kind::Enter,
            "Tab" => Kind::Tab,
            "Backspace" => Kind::Backspace,
            "Escape" => Kind::Escape,
            "ArrowUp" => Kind::Up,
            "ArrowDown" => Kind::Down,
            "ArrowLeft" => Kind::Left,
            "ArrowRight" => Kind::Right,
            "Home" => Kind::Home,
            "End" => Kind::End,
            "Insert" => Kind::Insert,
            "Delete" => Kind::Delete,
            "PageUp" => Kind::PageUp,
            "PageDown" => Kind::PageDown,
            "F1" => Kind::Function(Function::One),
            "F2" => Kind::Function(Function::Two),
            "F3" => Kind::Function(Function::Three),
            "F4" => Kind::Function(Function::Four),
            "F5" => Kind::Function(Function::Five),
            "F6" => Kind::Function(Function::Six),
            "F7" => Kind::Function(Function::Seven),
            "F8" => Kind::Function(Function::Eight),
            "F9" => Kind::Function(Function::Nine),
            "F10" => Kind::Function(Function::Ten),
            "F11" => Kind::Function(Function::Eleven),
            "F12" => Kind::Function(Function::Twelve),
            _ => {
                let mut chars = text.chars();
                let ch = chars.next().ok_or(InputError::UnsupportedKey)?;
                if ch.len_utf16() != 1 || chars.next().is_some() {
                    return Err(InputError::UnsupportedKey);
                }
                Kind::Character(ch)
            }
        };
        Ok(Self(kind))
    }
}
