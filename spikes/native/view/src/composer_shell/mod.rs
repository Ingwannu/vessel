//! Composer surface rules shared by native hosts, without editing or I/O.
//! Attachment state remains in `composer`; this module owns presentation policy.

pub mod catalog;
pub mod commands;
pub mod images;
pub mod links;
pub mod sending;
pub mod surface;
