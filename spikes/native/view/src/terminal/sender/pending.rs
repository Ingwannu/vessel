//! A retained UTF8 blob uses a consumed prefix, avoiding quadratic tail copies.
use super::{Outgoing, SenderError};
use vessel_proto::terminals::SendTerminal;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Characters(usize);
impl Characters {
    pub(super) const fn get(self) -> usize {
        self.0
    }
    pub(super) fn of(text: &str) -> Self {
        Self(
            text.chars()
                .take(super::MAX_PENDING.saturating_add(1))
                .count(),
        )
    }
    pub(super) fn add(&mut self, value: Self) {
        self.0 = self.0.saturating_add(value.0);
    }
    pub(super) fn remove(&mut self, value: Self) {
        self.0 = self.0.saturating_sub(value.0);
    }
}
#[derive(Debug, Default)]
struct ConsumedBytes(usize);
#[derive(Debug)]
pub(super) struct Keys {
    text: String,
    consumed: ConsumedBytes,
}
#[derive(Debug)]
pub(super) enum Pending {
    Keys(Keys),
    Resize { cols: u16, rows: u16 },
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Remainder {
    Empty,
    More,
}
pub(super) struct Batch {
    pub(super) outgoing: Outgoing,
    pub(super) remainder: Remainder,
    pub(super) chars: Characters,
}
impl Pending {
    pub(super) fn keys(text: &str) -> Result<Self, SenderError> {
        let mut owned = String::new();
        owned
            .try_reserve_exact(text.len())
            .map_err(SenderError::Allocation)?;
        owned.push_str(text);
        Ok(Self::Keys(Keys {
            text: owned,
            consumed: ConsumedBytes::default(),
        }))
    }
    pub(super) fn can_append(&self) -> bool {
        matches!(
            self,
            Self::Keys(Keys {
                consumed: ConsumedBytes(0),
                ..
            })
        )
    }
    pub(super) fn append(&mut self, value: &str) -> Result<(), SenderError> {
        let Self::Keys(keys) = self else {
            return Err(SenderError::InvalidQueue);
        };
        if keys.consumed.0 != 0 {
            return Err(SenderError::InvalidQueue);
        }
        keys.text
            .try_reserve(value.len())
            .map_err(SenderError::Allocation)?;
        keys.text.push_str(value);
        Ok(())
    }
    pub(super) fn take(&mut self) -> Result<Batch, SenderError> {
        match self {
            Self::Resize { cols, rows } => Ok(Batch {
                outgoing: Outgoing::Resize {
                    cols: *cols,
                    rows: *rows,
                },
                remainder: Remainder::Empty,
                chars: Characters::default(),
            }),
            Self::Keys(keys) => keys.take(),
        }
    }
}
impl Keys {
    fn take(&mut self) -> Result<Batch, SenderError> {
        let remaining = self
            .text
            .get(self.consumed.0..)
            .ok_or(SenderError::InvalidQueue)?;
        let mut end = remaining.len().min(SendTerminal::MAX_BYTES);
        while !remaining.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        if end == 0 {
            return Err(SenderError::InvalidQueue);
        }
        let prefix = remaining.get(..end).ok_or(SenderError::InvalidQueue)?;
        let mut payload = String::new();
        payload
            .try_reserve_exact(prefix.len())
            .map_err(SenderError::Allocation)?;
        payload.push_str(prefix);
        let consumed = self
            .consumed
            .0
            .checked_add(end)
            .ok_or(SenderError::InvalidQueue)?;
        let chars = Characters::of(prefix);
        let remainder = if consumed == self.text.len() {
            Remainder::Empty
        } else {
            Remainder::More
        };
        self.consumed = ConsumedBytes(consumed);
        Ok(Batch {
            outgoing: Outgoing::Keys(payload),
            remainder,
            chars,
        })
    }
}
