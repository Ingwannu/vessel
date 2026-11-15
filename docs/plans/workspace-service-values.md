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

Only the contract and plan are present. Value source and rows are next;
no method, SDK, setting, process or public route is implemented here yet.
