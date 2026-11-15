# W17 shared CLI output seam (early slice)

Base: announced node ba45a6e3. This is a standalone subset of W17, with no
host/capture/provider contracts, registry or parser implementation.

[Decision Log]
- Purpose: unblock independent permit (W4) and plugin (W16) CLI modules while
  preserving one table/JSON/YAML/quiet formatter.
- Existing assets: W17's typed presentation flags and Row/list/render_list/single
  helpers. Connection aliases and runtime failure classification are excluded.
- Alternatives: duplicating formatters would drift; waiting for the full W17
  package would unnecessarily couple consumers to unrelated node contracts.
- Choice: reuse these leaf modules inside vessel-cli, without a new crate or
  dependency. Configured `models` is the first real, read-only consumer.
- Impact: models keeps its default text. Explicit --json/-o json/-o yaml, -q and
  --no-headers use typed output. These flags do not imply format coverage for
  every legacy command. YAML is a YAML 1.2 document containing JSON flow syntax.
  No node calls or authority changes are introduced.

API (pub(crate), no formatter copying):
- args::parity::{OutputOptions, Presentation}
- args::parity::output::Headers
- parity::output::{Row, list, render_list, single}

`list` and `single` write data to stdout and return CliError; render_list returns
bytes for tests and owner-specific composition. Row requires Serialize plus an
ID string and table headers/cells. single's table fallback is pretty JSON;
commands wanting a key/value inspection table must supply their own row shape.
Quiet precedes --json, which precedes -o; --no-headers affects only tables.
Terminal control characters in table cells are escaped. Structured strings retain
serializer escaping. Runtime errors and connection selection remain owner policy.

Validation results are recorded below. Full W17 integration/package proof and
production account flows are separate from this early shared slice.

V3 return 1 correction: global presentation selection is tracked independently
from its default. Dispatch refuses unsupported explicit selections before even
connection parsing. Default legacy commands and no-color remain usable. Compare's
existing --json is preserved; other compare formats and raw-stream selections
are refused. Format strings now trim whitespace and ignore ASCII case as Paseo's
normalizeFormat does. Added root clap graph/admission and actual CLI regressions.

Corrected-head 872 targeted run compiled no tests: cargo failed E0599 because
admission used SessionCommand::Compare instead of the actual nested
SessionCommand::Conversation(ConversationCommand::Compare). Corrected both
product and test patterns, preserving the real `session compare` CLI spelling.
Evidence: /tmp/work/wp-cli/output-seam-tests. No runtime pass from that run.

## Executed shared-slice evidence

Exact product/test source: 5305e47a7f24c968141ca032f12107c428145e11.
- Targeted root/parser/admission/formatter/model tests plus actual fake-node CLI
  process regression: 6 passed / 0 failed / 0 ignored; run
  wp-cli-narrow-20261115T100125Z-5305e47a, /tmp/work/wp-cli/output-seam-tests2.
- Strict CLI all-target Clippy with proto/web TS features: exit0; run
  wp-cli-narrow-20261115T104604Z-5305e47a, /tmp/work/wp-cli/output-seam-clippy3.
- Its prior queued strict run101545 was interrupted143 by the operator-approved
  VM resize/restart; logs in output-seam-clippy2 remain distinct, not a source
  failure or pass. T/coordinator authorized the single same-source replacement.
- Original872 compile failure is retained in output-seam-tests; no test ran at
  that source. The corrected nested enum path is in5305.

This evidence commit changes this document only. V3 final review and integrator
merge remain required for consumer readiness. No full package or live-service
validation is claimed. No service ownership/mode change was made; only private
fake fixtures ran, as the build account.
