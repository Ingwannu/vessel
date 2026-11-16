//! Pairing problems as choices, not a diagnostic sentence copied into UI.
//!
//! [Decision Log]
//! - Purpose: every refusal is readable in the selected language and changes
//!   with it; a decoded offer's diagnostic may contain sensitive input.
//! - Constraint: the wire's decoder and node-book validation retain their own
//!   errors/rules. The view only chooses the person's whole message.
//! - Choice: small exhaustive problem enums, displayed through cached Fluent
//!   messages. Original diagnostic strings are never interpolated into them.
//! - Why: the renderer cannot accidentally expose Display text, and plain
//!   errors remain field borrows while a frame is drawn.
//! - Cost: a new decoder variant requires an explicit classification/message.

use crate::localization::Translator;

mod offer;
pub use offer::{OfferRoute, OfferRoutes};

/// Why the pasted pairing link cannot be used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkProblem {
    /// No offer has been decoded before connecting.
    Missing,
    /// Something other than a pairing link was pasted.
    WrongKind,
    /// Its encoded fragment is damaged.
    Encoding,
    /// Its decoded offer is damaged or unsupported.
    Offer,
}

impl From<&vessel_proto::LinkError> for LinkProblem {
    fn from(error: &vessel_proto::LinkError) -> Self {
        match error {
            vessel_proto::LinkError::Prefix => Self::WrongKind,
            vessel_proto::LinkError::Encoding => Self::Encoding,
            // Decoder coordinates are diagnostics, not a recovery sentence.
            vessel_proto::LinkError::Offer { .. } => Self::Offer,
        }
    }
}

impl LinkProblem {
    /// The whole explanation and next action, without the decoder diagnostic.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.pairing();
        match self {
            Self::Missing => words.paste_first(),
            Self::WrongKind => words.link_wrong_kind(),
            Self::Encoding => words.link_encoding(),
            Self::Offer => words.link_offer(),
        }
    }
}

/// Why a machine name will not do. The book's parser still validates syntax;
/// both native and browser adapters share the meaning and displayed words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameProblem {
    /// Nothing typed.
    Empty,
    /// The machine's key was put in the name field.
    LooksLikeKey,
    /// A pairing link was put in the name field.
    LooksLikeLink,
    /// The node-book parser does not accept this name.
    Invalid,
    /// The book already contains the name.
    Taken,
}

impl NameProblem {
    /// The whole explanation for the name field, in the selected language.
    #[must_use]
    pub fn words(self, catalog: &Translator) -> &str {
        let words = catalog.pairing();
        match self {
            Self::Empty => words.name_empty(),
            Self::LooksLikeKey => words.name_is_key(),
            Self::LooksLikeLink => words.name_is_link(),
            Self::Invalid => words.name_invalid(),
            Self::Taken => words.name_taken(),
        }
    }
}

/// A pairing request failed. Host diagnostics remain at the link edge; these
/// categories name the recovery a person can take without leaking a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairProblem {
    /// The shared machine list fault keeps its safe location and problem.
    Book(crate::files::FileFault),
    /// The book rejected a name after the initial field validation.
    Name,
    /// The machine or credential could not be kept locally.
    Storage,
    /// A route failed or the one-use invitation could not be accepted.
    Connection,
    /// The current browser transport does not implement pairing.
    Browser,
    /// The app's link did not take the request (its queue full, or
    /// stopped): nothing reached the machine, so the same link still
    /// works.
    Unsent,
}

impl PairProblem {
    /// A whole recovery message in the selected language.
    #[must_use]
    pub fn words<'a>(&self, catalog: &'a Translator) -> std::borrow::Cow<'a, str> {
        let words = catalog.pairing();
        let label = match self {
            Self::Book(fault) => {
                return std::borrow::Cow::Owned(
                    crate::Lang::from_tag(catalog.locale().tag()).machines_unreadable(fault),
                );
            }
            Self::Name => words.failed_name(),
            Self::Storage => words.failed_storage(),
            Self::Connection => words.failed_connection(),
            Self::Browser => words.failed_browser(),
            Self::Unsent => words.failed_unsent(),
        };
        std::borrow::Cow::Borrowed(label)
    }
}

/// A visible failure prepared when it arrives or the language changes. The
/// shared book fault includes values, so it must not be formatted in Render.
#[derive(Debug, Clone)]
pub struct PreparedFailure {
    problem: PairProblem,
    words: std::sync::Arc<str>,
}

impl PreparedFailure {
    /// Prepare a failure against an initialized process catalog.
    #[must_use]
    pub fn new(problem: PairProblem, catalog: &'static Translator) -> Self {
        let words = std::sync::Arc::from(problem.words(catalog).as_ref());
        Self { problem, words }
    }

    /// Reword an existing failure without repeating the failed operation.
    pub fn translate(&mut self, catalog: &'static Translator) {
        self.words = std::sync::Arc::from(self.problem.words(catalog).as_ref());
    }

    /// Share the prepared sentence with a toolkit without allocating a copy.
    #[must_use]
    pub fn shared_words(&self) -> std::sync::Arc<str> {
        std::sync::Arc::clone(&self.words)
    }

    /// Borrow the already prepared sentence while a frame is drawn.
    #[must_use]
    pub fn words(&self) -> &str {
        &self.words
    }
}

/// A successful pairing notice, prepared outside painting and retranslated
/// without pairing again. Its machine and route stay with its meaning.
#[derive(Debug, Clone)]
pub struct PairedNotice {
    name: String,
    via: crate::Via,
    words: String,
}

impl PairedNotice {
    /// Prepare the whole status sentence once the machine answers.
    /// # Errors
    /// Returns a formatting failure if translation and English both fail.
    pub fn new(
        name: &str,
        via: crate::Via,
        catalog: &Translator,
    ) -> Result<Self, crate::localization::FormatError> {
        Ok(Self {
            name: name.to_owned(),
            via,
            words: paired_words(name, via, catalog)?,
        })
    }

    /// Reword the existing notice when a language choice changes.
    /// # Errors
    /// Returns a formatting failure, keeping the previous sentence intact.
    pub fn translate(
        &mut self,
        catalog: &Translator,
    ) -> Result<(), crate::localization::FormatError> {
        self.words = paired_words(&self.name, self.via, catalog)?;
        Ok(())
    }

    /// Borrow the prepared whole sentence while drawing.
    #[must_use]
    pub fn words(&self) -> &str {
        &self.words
    }
}

fn paired_words(
    name: &str,
    via: crate::Via,
    catalog: &Translator,
) -> Result<String, crate::localization::FormatError> {
    use crate::localization::PairingPairedVia;
    let via = match via {
        crate::Via::Direct => PairingPairedVia::Direct,
        crate::Via::Relay => PairingPairedVia::Relay,
    };
    catalog.pairing().paired(name, via)
}

#[cfg(test)]
mod tests;
