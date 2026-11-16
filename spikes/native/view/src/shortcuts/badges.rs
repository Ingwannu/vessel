//! Workspace number badges follow the effective index binding, not a second
//! list of defaults. Only a single held modifier can reveal them, after 150ms.
use super::{Platform, keys, sequence::KeySequence};
use std::time::Duration;

/// Reference delay before sidebar index hints appear.
pub const REVEAL_DELAY: Duration = Duration::from_millis(150);
/// Physical modifier identity after resolving the platform's primary key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// Control.
    Control,
    /// Command/Windows.
    Meta,
    /// Option/Alt.
    Alt,
}
/// Toolkit-independent snapshot of the physical modifier keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one independent state per physical modifier"
)]
pub struct HeldModifiers {
    /// Control.
    pub control: bool,
    /// Command/Windows.
    pub meta: bool,
    /// Option/Alt.
    pub alt: bool,
    /// Shift always suppresses workspace index badges.
    pub shift: bool,
}
impl HeldModifiers {
    fn only(self) -> Option<Modifier> {
        match (self.control, self.meta, self.alt, self.shift) {
            (true, false, false, false) => Some(Modifier::Control),
            (false, true, false, false) => Some(Modifier::Meta),
            (false, false, true, false) => Some(Modifier::Alt),
            _ => None,
        }
    }
}

/// A multi-step or multi-modifier binding has no single badge-reveal key.
#[must_use]
pub fn index_modifier(keys: &KeySequence, platform: Platform) -> Option<Modifier> {
    let mut chords = keys.chords();
    let chord = chords.next()?;
    if chord.key != "Digit" || chords.next().is_some() {
        return None;
    }
    let [meta, control, alt, shift] = keys::held(chord, platform);
    HeldModifiers {
        control,
        meta,
        alt,
        shift,
    }
    .only()
}

/// The app schedules only an admitted new hold; passive frames do not restart it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// No timer or visibility work.
    Unchanged,
    /// Cancel an old timer and hide.
    Hidden,
    /// Start one timer for this absolute monotonic deadline.
    Waiting(Duration),
}

/// No I/O or timers in view. The UI owns a cancellable task and supplies time.
#[derive(Debug, Default)]
pub struct Reveal {
    active: Option<Modifier>,
    deadline: Option<Duration>,
    visible: bool,
}
impl Reveal {
    /// Blur, hidden windows, bindings, capture and overlay changes reset a hold.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    /// Whether the admitted hold has completed its delay.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.visible
    }
    /// Update a real modifier snapshot against the current effective binding.
    #[must_use]
    pub fn update(
        &mut self,
        required: Option<Modifier>,
        held: HeldModifiers,
        enabled: bool,
        now: Duration,
    ) -> Change {
        let active = enabled
            .then(|| held.only())
            .flatten()
            .filter(|key| Some(*key) == required);
        if self.active == active {
            return Change::Unchanged;
        }
        self.clear();
        self.active = active;
        if active.is_some() {
            let deadline = now.saturating_add(REVEAL_DELAY);
            self.deadline = Some(deadline);
            Change::Waiting(deadline)
        } else {
            Change::Hidden
        }
    }
    /// A cancelled/replaced timer cannot reveal a newer hold early.
    pub fn reveal(&mut self, deadline: Duration, now: Duration) -> bool {
        if self.deadline != Some(deadline) || now < deadline {
            return false;
        }
        self.deadline = None;
        self.visible = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn modifier_identity_comes_from_the_effective_single_digit_template() {
        let keys = |text| KeySequence::parse(text).unwrap();
        assert_eq!(
            index_modifier(&keys("Mod+Digit"), Platform::Mac),
            Some(Modifier::Meta)
        );
        assert_eq!(
            index_modifier(&keys("Mod+Digit"), Platform::Other),
            Some(Modifier::Control)
        );
        assert_eq!(
            index_modifier(&keys("Alt+Digit"), Platform::Other),
            Some(Modifier::Alt)
        );
        for text in [
            "Ctrl+Shift+Digit",
            "Ctrl+Alt+Digit",
            "Ctrl+K Digit",
            "Ctrl+3",
            "Digit",
        ] {
            assert_eq!(index_modifier(&keys(text), Platform::Other), None, "{text}");
        }
    }
    #[test]
    fn delay_shift_reset_and_stale_timer_are_explicit() {
        let mut reveal = Reveal::default();
        let control = HeldModifiers {
            control: true,
            ..HeldModifiers::default()
        };
        let Change::Waiting(old) =
            reveal.update(Some(Modifier::Control), control, true, Duration::ZERO)
        else {
            panic!("new hold")
        };
        assert!(!reveal.reveal(old, REVEAL_DELAY.saturating_sub(Duration::from_millis(1))));
        assert_eq!(
            reveal.update(
                Some(Modifier::Control),
                control,
                true,
                Duration::from_millis(100)
            ),
            Change::Unchanged
        );
        assert!(reveal.reveal(old, REVEAL_DELAY));
        assert!(reveal.visible());
        assert_eq!(
            reveal.update(
                Some(Modifier::Control),
                HeldModifiers {
                    shift: true,
                    ..control
                },
                true,
                REVEAL_DELAY
            ),
            Change::Hidden
        );
        let Change::Waiting(new) = reveal.update(
            Some(Modifier::Control),
            control,
            true,
            Duration::from_millis(200),
        ) else {
            panic!("new hold")
        };
        assert!(!reveal.reveal(old, Duration::from_secs(1)));
        reveal.clear();
        assert!(!reveal.reveal(new, Duration::from_secs(1)));
    }
}
