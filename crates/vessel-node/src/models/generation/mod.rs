//! Staged generated-title preparation (ADR 0062). Shared-accessor wiring
//! removes cfg(test) on this module and supplies the production caller.
//! Routing, HTTP and actor mutation do not belong to the text edge.

mod client;
mod prepare;
mod text;
