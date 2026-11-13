//! QR encoding without a UI dependency. A matrix is as sensitive as its
//! source link: no serialization and a deliberately redacted Debug.

use qrcode::{Color, QrCode};
use std::{fmt, num::NonZeroU16};

/// A QR code could not be represented.
#[derive(Debug, thiserror::Error)]
pub enum QrProblem {
    /// The text cannot fit a QR code.
    #[error("the link cannot fit a QR code; copy it instead")]
    Encode(#[source] qrcode::types::QrError),
    /// The encoder returned an unsupported dimension.
    #[error("unsupported QR dimensions; copy the link instead")]
    Dimensions,
}

/// An in-memory matrix, with no quiet zone (the renderer supplies it).
pub struct Matrix {
    side: NonZeroU16,
    dark: Vec<(u16, u16)>,
}

impl Matrix {
    /// Encode transient text. Never call with a hidden credential.
    ///
    /// # Errors
    /// The text is too long or the encoded matrix cannot be represented.
    pub fn encode(text: &str) -> Result<Self, QrProblem> {
        let code = QrCode::new(text.as_bytes()).map_err(QrProblem::Encode)?;
        let side = u16::try_from(code.width())
            .ok()
            .and_then(NonZeroU16::new)
            .ok_or(QrProblem::Dimensions)?;
        let colors = code.to_colors();
        let mut dark = Vec::new();
        for y in 0..side.get() {
            for x in 0..side.get() {
                if colors.get(
                    usize::from(y)
                        .saturating_mul(usize::from(side.get()))
                        .saturating_add(usize::from(x)),
                ) == Some(&Color::Dark)
                {
                    dark.push((x, y));
                }
            }
        }
        Ok(Self { side, dark })
    }
    /// The number of modules across, before the quiet zone.
    #[must_use]
    pub const fn side(&self) -> NonZeroU16 {
        self.side
    }
    /// Dark modules as (column, row), no allocation or text exposure.
    pub fn dark(&self) -> impl Iterator<Item = (u16, u16)> + '_ {
        self.dark.iter().copied()
    }
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("QrMatrix(<redacted>)")
    }
}
