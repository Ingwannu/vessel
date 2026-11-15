//! The page's test cases for the conversation's view logic, run against
//! the app's port (ADR 0036: behaviour has one specification, checked on
//! both sides). Each file names the page's test it follows. The modules
//! are `cfg(test)` so their helpers are test code to the lints, as the
//! cases themselves are.

#[cfg(test)]
mod find;
#[cfg(test)]
mod fork;
#[cfg(test)]
mod outline;
#[cfg(test)]
mod permission;
#[cfg(test)]
mod plan;
#[cfg(test)]
mod question;
#[cfg(test)]
mod rewind;
#[cfg(test)]
mod rows;
#[cfg(test)]
mod session;
#[cfg(test)]
mod support;
#[cfg(test)]
mod terminal;
#[cfg(test)]
mod timeline;
#[cfg(test)]
mod window;
#[cfg(test)]
mod words;
