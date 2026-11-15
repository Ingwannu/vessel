//! ADR 0077 staged file mechanism; no auth/store/session orchestration.
//!
//! Filesystem capture/restore follows bounded immutable images. It must not
//! expose a working product entry point before safe I/O and journal tests pass.

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "skeleton: implement raw bounded capture through regular-file guards"]
    fn dirty_tracked_nonignored_untracked_and_binary_bytes_survive_capture() {
        // Proof obligation: implement capture with real temporary repository.
    }
    #[test]
    #[ignore = "skeleton: implement effective-image staging and per-file restore"]
    fn restore_preserves_nonroot_ownership_and_tightened_access() {
        // Proof obligation: implement nonroot permissions fixture.
    }
    #[test]
    #[ignore = "skeleton: implement containment, link and metadata preflight"]
    fn links_fifos_parent_swaps_and_unsafe_metadata_refuse_without_mutation() {
        // Proof obligation: implement adversarial path fixtures.
    }
    #[test]
    #[ignore = "skeleton: implement restore before-image and fingerprint rollback"]
    fn io_failure_rolls_back_only_our_writes_and_keeps_external_edits() {
        // Proof obligation: implement journal fault-injection fixture.
    }
}
