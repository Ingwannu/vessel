//! Up and Down in an empty composer walk the messages already sent in the
//! session, as a shell's history does (Paseo's `composer/recall.ts`,
//! `ComposerRecall`, and `stores/session-store.ts`,
//! `selectAgentSentMessages`).
//!
//! A message stays recalled only while the composer holds its exact text;
//! the first edit gives the arrows back to the caret. Up starts only from an
//! empty composer; within a recalled message of several lines the arrows
//! move between its lines first.

/// The messages a person sent, newest first, each once in a row (Paseo
/// skips empty ones and a message equal to the one after it).
#[must_use]
pub fn sent_newest_first<'a>(
    oldest_first: impl DoubleEndedIterator<Item = &'a str>,
) -> Vec<&'a str> {
    let mut sent: Vec<&str> = Vec::new();
    for text in oldest_first.rev() {
        if text.trim().is_empty() || sent.last() == Some(&text) {
            continue;
        }
        sent.push(text);
    }
    sent
}

/// An arrow key at the composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrow {
    /// Up: an older message.
    Up,
    /// Down: a newer one, then the empty composer.
    Down,
}

/// What the composer shows and where its caret is, when a key is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct At<'a> {
    /// The text.
    pub text: &'a str,
    /// The selection's start and end (bytes); equal when nothing is
    /// selected.
    pub selection: (usize, usize),
}

/// Which sent message the composer shows, if one (Paseo's
/// `RecalledMessage`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Recall {
    shown: Option<(usize, String)>,
}

impl Recall {
    /// Whether the composer shows a recalled message as it was recalled
    /// (then a leading `/name` in it does not open the command menu: it is
    /// sent as it is).
    #[must_use]
    pub fn showing(&self, text: &str) -> bool {
        self.shown.as_ref().is_some_and(|(_, shown)| shown == text)
    }

    /// The text the composer should show after `arrow`, or `None` to leave
    /// the key to the caret. `sent` is newest first, read only for a key
    /// recall acts on.
    pub fn press<'a>(
        &mut self,
        arrow: Arrow,
        at: At<'_>,
        sent: impl FnOnce() -> Vec<&'a str>,
    ) -> Option<String> {
        let (start, end) = at.selection;
        if start != end {
            return None;
        }
        let index = match &self.shown {
            Some((index, shown)) if shown == at.text => *index,
            _ => {
                self.shown = None;
                if arrow != Arrow::Up || !at.text.is_empty() {
                    return None;
                }
                return self.show(&sent(), 0);
            }
        };
        match arrow {
            Arrow::Up => {
                if !on_first_line(at.text, start) {
                    return None;
                }
                self.show(&sent(), index.checked_add(1)?)
            }
            Arrow::Down => {
                if !on_last_line(at.text, start) {
                    return None;
                }
                match index.checked_sub(1) {
                    None => {
                        self.shown = None;
                        Some(String::new())
                    }
                    Some(newer) => self.show(&sent(), newer),
                }
            }
        }
    }

    fn show(&mut self, sent: &[&str], index: usize) -> Option<String> {
        let text = (*sent.get(index)?).to_owned();
        self.shown = Some((index, text.clone()));
        Some(text)
    }
}

fn on_first_line(text: &str, caret: usize) -> bool {
    !text.get(..caret).unwrap_or(text).contains('\n')
}

fn on_last_line(text: &str, caret: usize) -> bool {
    !text.get(caret..).unwrap_or_default().contains('\n')
}

#[cfg(test)]
mod tests;
