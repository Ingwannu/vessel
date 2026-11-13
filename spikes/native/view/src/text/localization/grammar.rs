//! Korean particles are chosen from the value, not the label in today's
//! screenshot. Latin text uses the Korean names of its final letter
//! (L/M/N/R have a final consonant); it never guesses a word's pronunciation.

use fluent_bundle::{FluentArgs, FluentValue};

/// The Korean grammatical role selected by a message's `PARTICLE` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Particle {
    /// 을/를.
    Object,
    /// 이/가.
    Subject,
    /// 은/는.
    Topic,
    /// 과/와.
    With,
    /// 으로/로 (a final ㄹ uses 로).
    Direction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    Open,
    Consonant,
    Rieul,
}

impl Particle {
    /// Append after a value. Trailing punctuation/space is ignored. Hangul
    /// uses its actual final consonant; digits use Sino-Korean readings
    /// (0 영, 1 일, …); Latin uses letter names. Other scripts and an empty
    /// value use the open form. A final ㄹ is special only for Direction.
    #[must_use]
    pub fn after(self, value: &str) -> &'static str {
        let ending = ending(value);
        match (self, ending) {
            (Self::Object, Ending::Open) => "를",
            (Self::Object, Ending::Consonant | Ending::Rieul) => "을",
            (Self::Subject, Ending::Open) => "가",
            (Self::Subject, Ending::Consonant | Ending::Rieul) => "이",
            (Self::Topic, Ending::Open) => "는",
            (Self::Topic, Ending::Consonant | Ending::Rieul) => "은",
            (Self::With, Ending::Open) => "와",
            (Self::With, Ending::Consonant | Ending::Rieul) => "과",
            (Self::Direction, Ending::Open | Ending::Rieul) => "로",
            (Self::Direction, Ending::Consonant) => "으로",
        }
    }

    fn from_name(value: &str) -> Option<Self> {
        match value {
            "object" => Some(Self::Object),
            "subject" => Some(Self::Subject),
            "topic" => Some(Self::Topic),
            "with" => Some(Self::With),
            "direction" => Some(Self::Direction),
            _ => None,
        }
    }
}

fn ending(value: &str) -> Ending {
    let Some(last) = value.chars().rev().find(|c| c.is_alphanumeric()) else {
        return Ending::Open;
    };
    let point = u32::from(last);
    if let Some(hangul) = point.checked_sub(0xac00).filter(|n| *n <= 11171) {
        return match hangul.checked_rem(28) {
            Some(0) => Ending::Open,
            Some(8) => Ending::Rieul,
            Some(_) | None => Ending::Consonant,
        };
    }
    // A decomposed Hangul value ends in a jongseong, not a syllable.
    if point == 0x11af {
        return Ending::Rieul;
    }
    if (0x11a8..=0x11ff).contains(&point) {
        return Ending::Consonant;
    }
    match last.to_ascii_lowercase() {
        '1' | '7' | '8' | 'l' | 'r' | 'ㄹ' => Ending::Rieul,
        '0' | '3' | '6' | 'm' | 'n' | 'ㄱ'..='ㅎ' => Ending::Consonant,
        _ => Ending::Open,
    }
}

pub(super) fn fluent<'a>(
    positional: &[FluentValue<'a>],
    _named: &FluentArgs<'_>,
) -> FluentValue<'a> {
    let Some(FluentValue::String(kind)) = positional.get(1) else {
        return FluentValue::Error;
    };
    let Some(particle) = Particle::from_name(kind) else {
        return FluentValue::Error;
    };
    match positional.first() {
        Some(FluentValue::String(value)) => FluentValue::from(particle.after(value)),
        Some(FluentValue::Number(value)) => FluentValue::from(particle.after(&value.as_string())),
        _ => FluentValue::Error,
    }
}
