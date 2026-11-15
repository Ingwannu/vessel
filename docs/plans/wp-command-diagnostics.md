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
