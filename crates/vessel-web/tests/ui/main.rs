//! `vessel ui`'s server over real loopback sockets: what a browser on
//! this machine gets, and what a page elsewhere, a rebinding site or a
//! client without the token does not; then the bridge to real nodes in
//! this process (ADR 0017), the book's edits from the page, the page's
//! preferences and drafts (ADR 0028), and the HTML preview route (ADR
//! 0122).

#[cfg(test)]
mod book;
#[cfg(test)]
mod bridge;
#[cfg(test)]
mod prefs;
#[cfg(test)]
mod preview;
#[cfg(test)]
mod server;
#[cfg(test)]
mod support;
