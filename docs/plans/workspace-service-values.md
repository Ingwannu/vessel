# Workspace service consumer values

Status: implementation preparation under ADR 0066's reviewed consumer contract.

## Existing assets and decision

Modify existing proto/client owners. Reuse WorkspaceId, ScriptEntry,
ServicePort, CommandsDigest, TerminalInfo, method-table generation and typed
NodeClient::call. Use the already owned url parser; no new dependency or
crate edge. Existing generic URLs allow paths, transports or credentials
that a service origin must refuse, so a distinct validated origin type is
needed. Parsing carries the standard parser's input-free cause.

Implement on the announced node head in isolation from the unqualified
helper/Preview stack. The shared project command bound is consumed only
from its qualified owner; no new bound copy. Carry the already reviewed
ServiceEnvName owner unchanged if absent, never a second normalizer.

## Order and checks

1. Origin/key/etag and observation value owners with constructor/serde tests.
2. Bounded snapshot/environment/settings DTOs; the private disk model stays
   with the sole settings writer and awaits canonical physical incarnation.
3. Eight staged None rows, every explicit nonforwardable and agent-deny line,
   thin SDK wrappers, projection and admission tests. No handler activation.
4. Changed-crate strict checks/tests and generated TypeScript freshness,
   then source review. One complete gate at the enclosing feature boundary.

## Resume point

Origin/key/metadata etag values are authored, with canonical URL/serde,
forbidden-shape, bound and safe-source-chain tests. The parser uses the
existing url dependency and retains input-free typed sources. URI repairs
(dot paths/backslashes), empty userinfo and credentials refuse before any
metadata value is created. No network work occurs. DTO collections, method
rows and SDK wrappers are next; no setting, process or route is implemented.
Narrow strict/TS-feature tests on this exact source remain owed.


Observation values now preserve explicit unplanned/reserved/unavailable,
health pending/unknown and Planned versus Registered URL metadata. Both URL
roles survive; preference is derived. Empty/mislabelled links refuse,
including a public slot claiming loopback or node-loopback metadata naming a
non-loopback origin. Retain the parsed URL owner rather than reparse text
for that classification; the predicate performs no DNS or socket work.

Rows keep the current ScriptEntry owner separate from an older terminal's
command/type. Paseo script-status-projection.ts100-164,214-249 uses the
current definition with the latest runtime entry keyed by name. Matching
commands/kinds would reject legitimate edits while the earlier approved run
continues. Only recorded name association is required; plain definitions
cannot carry service fields. RemovedRunning requires a supervised terminal
observation, never proof of kernel liveness. Complete row view destructures
every owner field. Six more constructor/serde/projection regressions are
authored; none executed yet. No row/health/URL grants runtime authority.

Next: consume the announced shared MAX_PROJECT_COMMANDS owner for bounded
page/env DTOs. Staged rows and SDK wrappers remain absent. Existing exact
origin94811 suite stays pinned; later observation source needs its own
changed-head narrow suite. No previous result qualifies this commit.


Dedicated Admin values are now authored: literal nonzero-port gateway
endpoint, credential-free DNS public base, bounded/unique settings view,
explicit anonymous warning acknowledgement, optimistic revision requests,
and distinct saved/applying/applied/refused outcomes. Constructors and
serde share the same validation; no DNS, network or private-slot IO occurs.
The settings value is an Admin projection, NOT StoredServiceConsent or a
SavedNodeSettings disk payload. The required canonical incarnation remains
absent, and the real enable handler must return Unavailable before writing.
No generic settings field, writer, method row or SDK is introduced here.

The publication count bound moved beside ServicesAdminSettings and its
numeric documentary copy was removed. Six more boundary/serde tests are
authored, including missing/implicit acknowledgement, duplicate/over-count
refusal and DNS versus loopback role. All TCP boundary cases now derive
invalid ports from ServicePort::MIN/MAX; sample non-bound ports are unchanged.
A row-view doc reference is marked as code; no runtime behavior changed.
All execution for this changed head remains owed; earlier immutable source
checks continue untouched and cannot qualify these additions.


URL-role correction: IPv4-mapped IPv6 loopback is node-loopback metadata
rather than anonymous-public metadata. The same already parsed IPv6 address
uses std Ipv6Addr::to_ipv4_mapped and the mapped IPv4 predicate; native ::1
keeps its existing loopback classification. This adds no resolver/parser,
connection or kernel/route attestation. Constructor and serde role fixtures
cover both mapped 127/8 examples, ordinary ::1, documentation IPv6 and mapped
non-loopback IPv4. The old canonical/credential/dot-path/cause tests remain.
Reference: [Rust IPv6 embedding rules](https://doc.rust-lang.org/std/net/struct.Ipv6Addr.html#embedding-ipv4-addresses).
All eighteen authored consumer-value tests remain unexecuted on this source.
