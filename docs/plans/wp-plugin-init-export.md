# W16 local plugin init export

This is a portable extraction of the local CLI leaf reviewed under row238,
based on announced node headbc9cacc6. It excludes the private node admission,
artifact/source/candidate/settings code and all native app ancestry. No public
plugin method or installation capability is assigned by this export.

[Decision Log]
- Purpose: make the usable local author scaffold reviewable independently of
  the still incomplete F6 host and installer.
- Reuse: canonical proto PluginId/ClientDocument, W17 presentation admission and
  Row/list output, std exclusive filesystem operations and existing CLI dev deps.
- Alternatives: import the private development branch or emit React/npm sources.
- Choice: carry only the already reviewed CLI source and add complete canonical
  test clocks around its actual subprocess cases.
- Why: init needs no node discovery, credentials, helper or downloaded toolchain;
  its local behavior can be qualified without importing unrelated authority.
- Impact: one implemented `plugin init` command; all management leaves remain
  absent. There is no manifest/dependency/lock update in this extraction.

The exported production CLI source matches the selected row238 implementation
through d9144388. It emits fixed native manifest/client/guide files, validates
identity/templates before mutation, requires an empty ordinary target, and uses
create_new for every fixed file. Dot targets use the standard absolute-path
operation. Existing content/modes are preserved; new Unix targets/files request
0700/0600 subject to umask. Missing ancestor creation uses ambient umask and is
not a proof that every ancestor is private. Failure may leave a partial scaffold;
recursive rollback could delete concurrent work and is deliberately excluded.
Same-account path replacement and arbitrary filesystem durability are unqualified.

Dispatch runs the existing presentation admission before the local leaf. Its
malformed --node subprocess fixtures prove no connection is needed; env_clear
and temporary HOME/TMP isolate each case. Existing output modes, dot-directory,
invalid identity, no-overwrite, symlink and numeric-owner/mode assertions are
retained. The three asynchronous subprocess cases now use canonical within
with a named180s whole-case budget; each actual CLI invocation retains its
existing named30s phase limit and kill_on_drop behavior. Fakes are prepared by
within before its clock. The already declared CLI testkit dev edge is reused.

Prior evidence remains historical: source05288eae executed112 binary units plus
3 actual subprocess tests; d9144388 corrected only style/receipts and passed
strict. The portable source, new whole-case wrappers and current receiving union
require their own exact tests/strict/review. No current test, whole F6, installer,
NodeHost, Windows runtime or native UI approval is inferred from that history.

Reference: Paseo CLI scaffold170-235/index214-218. The native static author format
is deliberate; generating this directory does not install, enable, execute, build
or download a plugin.
