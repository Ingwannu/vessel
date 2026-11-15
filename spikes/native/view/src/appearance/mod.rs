//! Local Appearance preference and draft rules, with no renderer or I/O.
//!
//! [Decision Log]
//! - Purpose: validate family/size/syntax drafts before committing preferences,
//!   while a local preview can differ from the saved value.
//! - Constraint: the existing preference store and app's one palette adapter
//!   remain authoritative; plugin transport belongs to the plugin runtime.
//! - Choice: typed input and draft transitions here, tokens in kit, persisted
//!   changes through the existing PreferenceChange/Prefs path.
//! - Effect: input validation and preview/commit semantics are unit-testable
//!   without constructing a window or starting a node.

mod family;
pub mod form;
mod input;
mod number;
mod syntax;
mod width;

pub use family::{FaceName, FamilyError, FamilyFace, FontFamily, GenericFamily, MAX_FAMILY_UNITS};
pub use input::DraftText;
pub use number::Number;
pub use syntax::SyntaxTheme;
pub use width::{ContentPixels, WidthError};
