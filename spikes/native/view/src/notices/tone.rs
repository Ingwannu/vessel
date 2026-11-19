//! A notice's sound (ADR 0114): the page's two tones
//! (`web/src/features/notifications/sound.ts`), as samples the app plays.
//!
//! [Decision Log]
//! - Purpose: the app sounds the same as the page. The system's
//!   notification is silent on both platforms, and "Sound only" has no
//!   notification at all, so the app plays the sound itself.
//! - Choice: the page's Web Audio graph, computed here:
//!   - two sines, the second starting 0.16 s after the first;
//!   - needs-you rises 660 → 880 Hz, finished falls 880 → 660 Hz;
//!   - each tone's gain rises exponentially from 0.0001 to 0.18 in 0.02 s,
//!     falls back to 0.0001 at 0.22 s and holds there, and the tone stops
//!     at 0.25 s (`exponentialRampToValueAtTime`, `stop`).
//! - Samples are `f64`, mono. The player converts them to its device's
//!   format and channels, so no format is decided here.
//! - Kept pure, so the sound is tested without a device.

use super::NoticeKind;

/// How long a notice's sound lasts, in hundredths of a second: the second
/// tone starts at 0.16 s and stops 0.25 s later.
const HUNDREDTHS: u32 = 41;

/// One tone: when it starts and its pitch.
struct Tone {
    at: f64,
    hertz: f64,
}

/// The sound of `kind` at `rate` samples a second. Empty when `rate`
/// gives no sample.
#[must_use]
pub fn samples(kind: NoticeKind, rate: u32) -> Vec<f64> {
    let (first, second) = match kind {
        NoticeKind::NeedsYou => (660.0, 880.0),
        NoticeKind::Finished => (880.0, 660.0),
    };
    let tones = [
        Tone {
            at: 0.0,
            hertz: first,
        },
        Tone {
            at: 0.16,
            hertz: second,
        },
    ];
    let count = rate
        .checked_mul(HUNDREDTHS)
        .and_then(|scaled| scaled.checked_div(100))
        .unwrap_or(0);
    let rate = f64::from(rate);
    (0..count)
        .map(|index| {
            let time = seconds(index, rate);
            tones.iter().map(|tone| tone.sample(time)).sum()
        })
        .collect()
}

#[expect(clippy::float_arithmetic, reason = "a sample's time")]
fn seconds(index: u32, rate: f64) -> f64 {
    f64::from(index) / rate
}

impl Tone {
    /// Its value at `time` seconds into the sound.
    #[expect(clippy::float_arithmetic, reason = "a sine under its gain")]
    fn sample(&self, time: f64) -> f64 {
        let into = time - self.at;
        if !(0.0..0.25).contains(&into) {
            return 0.0;
        }
        gain(into) * (std::f64::consts::TAU * self.hertz * into).sin()
    }
}

/// The page's gain at `into` seconds into a tone.
#[expect(clippy::float_arithmetic, reason = "an exponential ramp")]
fn gain(into: f64) -> f64 {
    const QUIET: f64 = 0.0001;
    const LOUD: f64 = 0.18;
    if into <= 0.02 {
        QUIET * (LOUD / QUIET).powf(into / 0.02)
    } else if into <= 0.22 {
        LOUD * (QUIET / LOUD).powf((into - 0.02) / 0.2)
    } else {
        QUIET
    }
}
