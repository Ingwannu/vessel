# Native field controls: standalone producer export

This branch exports only the shared TextField drawing split and additive
selection/Settings input profile, above the announced package base 3.
It does not import Settings consumers, machine-book operations or other
controls. Shared history is preserved rather than rewriting the original
combined commit. W4 remains the sole owner of events/Editability.

The first commit moves rendering and token projection, preserving factory,
registry, composer hook and editing behavior. The field root falls below
500 lines and leaves the size exception list. Presentations/styles keep
one renderer/projection each. Existing constructors and tests are unchanged.

Source provenance: the first move is the exact merged field source from
955bae14, incorporating original move0cee06de plus W4 from6036eeb8. Original
0cee narrow kit Clippy passed and unchanged field tests passed16/0/0; this
export is not a separate whole-gate or UI approval. The following additive
commit will record exact compiled-source equivalence and pending tests.
