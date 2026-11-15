# Private command diagnostics

Purpose: terminal profiles may substitute a private prompt into both executable
text and argv. Plugins also pass private paths/arguments. The existing process
description hides environment values but intentionally shows program/argv;
proc and PTY launch logs/errors can therefore reveal a private launch.

Modify/reuse the existing CommandSpec and process/PTY error boundaries. A node
wrapper alone cannot hide a lower-level spawn log. Globally redacting every
command would remove existing diagnostics unnecessarily. A typed opt-in
CommandDiagnostics policy keeps current callers unchanged and redacts a private
spec's Debug, executable log label and returned spawn error payload/source.
No shell quoting, program resolution, execution, group/identity, stream or
reaping behavior changes. Standard-library formatting/error-kind projection
and existing tracing suffice; no dependency or wire type is added.

The actual execution fields remain intact. Diagnostic labels must never be
used to execute a command. Private error projection keeps a safe variant and
I/O kind while dropping backend text/chains that may contain command content;
default errors retain their original source. PTY projection covers every error
variant, including setup/attach/thread failures. Child program output and the
OS-visible command are execution data, outside this diagnostic policy.

Validate default diagnostics, unchanged execution fields, sentinel absence in
private Debug and complete error chains, and actual proc/PTY spawn log events.
Use the existing fake binary only; no real provider, credentials or environment
mutation. Tests and strict checks run on the build VM against the isolated
announced-head leaf. Source review and integrator adoption are still owed.


[Decision Log]
- Purpose: portable-pty 0.9.0's Windows command builder traces inherited system
  and user environment values; these may be credentials even for ordinary specs.
- Existing constraint: the daemon installs the log-to-tracing bridge through
  tracing-subscriber's initializer. An unrestricted RUST_LOG=trace admits those
  upstream records independently of CommandSpec's opt-in policy.
- Alternatives: patching the backend or suppressing all trace diagnostics would
  broaden this change and affect unrelated callers. An appended EnvFilter
  directive alone can be overridden by a more specific user target directive.
- Choice: after the user's filter, apply one independent fixed Targets directive
  portable_pty=warn with trace as the default for other targets. The two filters
  intersect, so a user directive cannot lower the portable-pty ceiling.
- Why: tracing-log checks the record's original target before forwarding it;
  a child-process regression exercises that real bridge and binary initializer
  with RUST_LOG=trace and an explicit more-specific trace override. Warn remains
  available and unrelated trace events remain available.
- Impact: portable-pty debug/info/trace diagnostics are suppressed on every
  platform, including ordinary commands. Child output, OS process visibility,
  other embedded consumers' logging setup and unrelated backend warnings remain
  outside this cap and must not be described as globally secret-free.

The test-only log dependency already exists transitively; it lets the fixture
emit actual log records without adding a production dependency or a second
logger. The initial strict run found only two format_push_string test lints;
those fixtures now write into the existing buffer without an extra allocation.
