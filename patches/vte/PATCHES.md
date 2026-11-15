# VTE 0.15.0 metadata patches

[Decision Log]
- Purpose: preserve information already decoded by the one terminal interpreter.
- Constraint: normalized parameters lose omitted-versus-zero Kitty negotiation;
  raw observers otherwise need a competing scanner to find complete tokens.
- Alternatives: guessed defaults, regex filtering, stdin response suppression.
  They break negotiation or suppress real keys and duplicate interpretation.
- Choice: read-only metadata in the existing decode pass; no decoder replacement.
- Cost: path-patched dependency requires archive comparison and pure audit.

Published archive and all original member hashes are recorded beside this file.
LICENSE-MIT and LICENSE-APACHE remain byte-identical to that archive.

1. Retain scalar parameter presence alongside the unchanged normalized values.
   Why: bare CSI > u and explicit CSI > 0 u have different Kitty defaults.
   Existing Perform callbacks receive the same numeric values and grouping.
2. Expose completion and prior-prefix boundaries while using the original
   decoder transitions. Why: presentation must withhold partial UTF-8/control
   tokens and distinguish cancellation/OSC termination from the following ESC
   command. No raw byte scanner, parser replay or mutable restoration is added.

No runtime, build script, I/O or upstream submission is introduced.
