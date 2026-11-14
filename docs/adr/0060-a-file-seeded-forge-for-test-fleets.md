# ADR 0060: A file-seeded forge for test fleets

Status: proposed for the test fleet; no wire or production sign-in change.

[Decision Log]
- Purpose: a disposable fleet must show real node PR facts without a forge
  account or network. The existing FakeForge already drives node E2E; use
  that implementation, not a second simulator or fake HTTP provider.
- Existing constraints: ForgeSetup.custom injects a provider without asking
  for credentials. vessel-testkit is a node dev-dependency, so making it
  depend on the node would introduce a cycle. Product vesseld must never
  honor a fake-provider flag or environment variable.
- Choice: a separate vessel-forge-fixture executable in vessel-node,
  required-features = forge-fixture, forwarding vessel-forge/fake. Its own
  argument parser takes a seed file, explicit scratch home/programs/data
  directories and loopback bind/public URLs. Product vesseld's parser and
  environment handling remain unchanged, including when that feature is on.
  The launcher uses the ordinary Node, admission, store and ForgeSetup.custom;
  its harness lookup is the supplied fixture programs directory only.
- Why: a feature-gated test executable avoids the dependency cycle and keeps
  every product entry point real. Paseo's github-service.test.ts injects
  program answers; Vessel already has the narrower provider test double in
  node E2E/forge.rs. Exposing the double through a separate test launcher
  makes the same core behavior capturable, without copying reference code.

The UTF-8 JSON seed has version 1 and repositories. Each repository contains
its typed RepoRef (kind, host, path), optional failure, and pulls. Each pull
contains a complete existing wire PullRequest under pull, and merge set to
normal or refused (normal by default). The pull's summary.repo must equal
its containing repository; summary.checks must equal checks.state. The head
branch supplies FakeForge::put's branch, so it is not specified twice.
Existing wire newtypes/enums validate identifiers, titles, URLs and states.
Unknown seed fields/versions, duplicate repositories/pull numbers and
inconsistent summaries refuse the whole file. No executable code or tokens
are in the format; examples use reserved fixture hosts and local git pushes.

Limits: one regular non-symlink file, at most 1 MiB; 16 repositories, 128
pulls total, at most Checks::MAX_FAILING named failing checks per pull and
three distinct merge methods. No partial loading. Parsing/I/O happens off
async workers before listening. Diagnostics give category/location, never
seed text or the offending value; typed sources are retained without printing
their potentially sensitive text. The file is read once; restart to change it.

The loader seeds real FakeForge instances/state. Per-repository no_access,
rate_limited, down and signed_out failures use the same ForgeError mapping
as its existing global failure. Whoami remains the fake's own login; no gh
or HTTP fallback is attempted. A refused merge preserves facts and returns
Merged::Rejected after the normal head check. The seeded fake rejects unknown
repositories, caps runtime pull creation at 128 and retains bounded merge
attempt history; its conditional caches retain only existing pull/branch
keys. Opening allocates past seeded numbers, without overwrite or overflow.

fleet.sh prepares fixture repositories and seed files before startup, uses
this executable for its two disposable nodes, pairs through CLI pipes with
person grants, then creates placed fake sessions and checks PR facts/refusals
through the ordinary CLI. Push URLs name local bare repositories. The
standalone no-route diagnostic continues to use product vesseld. Build the
fixture executable explicitly; default release builds do not include it.

Deliberately not simulated: network/HTTP/ETags/rate-header parsing, real token
lookup or OAuth, remote git pushes/merges, and review discussion/feedback
content. These keep their existing provider/integration tests. Fake facts
and explicit failures are fixture behavior, not production/network proof.
Tests exercise actual FakeForge trait methods, file bounds/diagnostic privacy,
real node/CLI projection, and rejection of fixture flags by product vesseld.
