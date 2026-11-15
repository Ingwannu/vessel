//! Private service/guard proof while the feature has no protocol assignment.
//! The real pending RPC refusal is tested separately: these are not happy-wire claims.
mod authority;
mod fixture;
mod ownership;
mod probes;
mod protocol;
mod runtime;
use fixture::*;
