//! A session's model calls through the node's proxy (protocol 7), summed
//! as they arrive, so the inspector's totals cost nothing per frame (the
//! page's `data/models.ts`, `addCall`).

use vessel_proto::{ModelCall, ModelId};

/// A session's model calls, summed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelUsage {
    /// Calls made.
    pub calls: u64,
    /// Calls that failed.
    pub failed: u64,
    /// Input tokens outside the prompt cache.
    pub input: u64,
    /// Input tokens read from the cache.
    pub cache_read: u64,
    /// Input tokens written to the cache.
    pub cache_write: u64,
    /// Output tokens, reasoning included.
    pub output: u64,
    /// Reasoning tokens.
    pub reasoning: u64,
    /// Time spent, in milliseconds.
    pub ms: u64,
    /// The upstream models that answered, in first-call order.
    pub models: Vec<ModelId>,
}

impl ModelUsage {
    /// `call` added. Sums saturate: a log long enough to spend `u64` says
    /// the most it can rather than wrap.
    pub fn add(&mut self, call: &ModelCall) {
        self.calls = self.calls.saturating_add(1);
        if call.failure.is_some() {
            self.failed = self.failed.saturating_add(1);
        }
        let usage = &call.usage;
        self.input = self.input.saturating_add(usage.input.get());
        self.cache_read = self.cache_read.saturating_add(usage.cache_read.get());
        self.cache_write = self.cache_write.saturating_add(usage.cache_write.get());
        self.output = self.output.saturating_add(usage.output.get());
        self.reasoning = self.reasoning.saturating_add(usage.reasoning.get());
        self.ms = self.ms.saturating_add(call.duration_ms.get());
        if !self.models.contains(&call.model) {
            self.models.push(call.model.clone());
        }
    }

    /// Every input token, cached or not.
    #[must_use]
    pub const fn total_input(&self) -> u64 {
        self.input
            .saturating_add(self.cache_read)
            .saturating_add(self.cache_write)
    }
}
