# Pure terminal-profile dependency

This root-only producer starts on announced node head82ae7c91 and contains
only the terminal_profiles module, its module registration and this note.
The module blobs are byte-identical to5c39da5f, the pure value portion inspected
by V3 row125 and preserved as separately reviewed by row129. Its prior narrow
evidence is six passing value tests and strict proto all-target Clippy. The
base-only port has no fresh check or adoption claim until its explicit review
disposition and node integration route are recorded.

The values bound profile identities, names, program/argv templates, prompts,
per-string and aggregate sizes. They substitute the literal sentinel without
shell interpolation, omit whole empty prompt arguments/assignments, reject
reserved or duplicate ids, and redact command/prompt values in diagnostics.
Defaults are data; they neither detect installations nor authorize execution.

The design reuses validating protocol values instead of importing the App
creation/composer stack or adding another profile/draft identity. No external
library, setting write, receipt/store schema, RPC row, node handler, launcher,
App file, native file or active protocol version is added. The optional package
version stays None. The remaining receipt/profile feature is a separate package.
