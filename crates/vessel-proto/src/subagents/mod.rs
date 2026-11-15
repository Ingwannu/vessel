//! Validated provider source values shared by child navigation and pane keys.
//! No method, event, authorization change or provider process is enabled here.
mod identity;
mod values;
pub use identity::ProviderChildRef;
pub use values::{ProviderChildId, ProviderText, SubagentValueError};
