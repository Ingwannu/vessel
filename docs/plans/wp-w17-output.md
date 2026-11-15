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

Validation pending: narrow CLI clippy and shared presentation/formatter tests.
No consumer-ready claim, full package gate or production account test yet.
