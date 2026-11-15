//! Browser execution primitives, below node principal/workspace admission.
//!
//! No engine is qualified or launched by this staged library. Networking
//! validates the whole resolved answer and dials only a checked socket;
//! references cannot alias elements across snapshot/navigation replacement.
//! The node remains responsible for authenticated owner/service capabilities.
//! A10 staging: the planned `browser: wire the qualified node browser actor`
//! commit removes these attributes only with its real node consumer, qualified
//! runtime and reviewed service authority. No caller-less product API is exported.

#[cfg(test)]
mod cdp;
#[cfg(all(test, target_os = "linux"))]
mod launch;
#[cfg(test)]
mod network;
#[cfg(test)]
mod page;
#[cfg(test)]
mod preparation;
#[cfg(all(test, target_os = "linux"))]
mod profile;
#[cfg(test)]
mod proxy;
#[cfg(test)]
mod references;

#[cfg(test)]
mod snapshot;

#[cfg(test)]
mod input_lane;
