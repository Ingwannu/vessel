# Browser execution staging

This crate does not provide a supported browser engine. Every execution module
is behind the literal outer `cfg(test)` boundary required by A10. Browser
methods and the native browser pane are unavailable on this branch.
A fake CDP peer, a synthetic PNG or an echoed evaluate input cannot change that.

The design and its deliberate differences from Paseo are in
[ADR 0078](../../docs/adr/0078-browser-engine-and-native-panel.md) and the
[package plan](../../docs/plans/impl-browser.md). The node actor wiring may
remove General staging with its reviewed caller and runtime qualification under
[ADR 0119](../../docs/adr/0119-general-browser-activation-independent-of-preview.md).
Preview requires separate service authority.

## Responsibilities

| Module | Owns | Does not grant |
| --- | --- | --- |
| `cdp` | Bounded sealed requests, response/session correlation, private framing, retained driver join | A caller-selectable method, realm or debugger endpoint |
| `cdp::client::owner` | Linux candidate process, bundle attestation, private pipe and stderr drain through shutdown | Confirmation that every descendant has exited or that the profile can be reused |
| `delivery` | One atomic winner between pending withdrawal and the first effect | Permission to replay an uncertain command |
| `page` | One target's revision, geometry, refs and pressed input; cancellation quarantine | Workspace, destination or runtime authority |
| `input_lane` | Bounded registered profile input work, activation/input ordering and retained stop | A real driver or a process-death receipt |
| `preparation` | Finite tracked blocking DNS/interface/file inspection through actual completion | Early capacity release when a waiter times out |
| `network` | Complete public-answer/interface classification and pinned TCP peer | Private Preview reach or UDP isolation |
| `proxy` | HTTP/CONNECT/upgrade forwarding, header sanitation, owned stream/task drain | A DIRECT fallback, browser qualification or TLS inspection |
| `references` | Monotonic reserved namespaces and terminal discard | Reuse of an old ref after navigation or a failed traversal |
| `snapshot` | Independently written bounded DOM traversal, typed decoding and rendering | Reference implementation code or complete-DOM/paint evidence |
| `profile` | Linux private directories, no-follow regular lock and explicit release | Windows ACL support or a cleanup certificate |
| `launch` | Fixed candidate argv/environment, read-only binary/shell/bundle observations | A qualified runtime, product launcher or supervisor |

The browser wire is pending and the product does not register browser methods.
The intended flow is node admission → retained profile/command owner → typed
page/CDP request → actual owned child pipe. General traffic must pass the policy
proxy. Preview additionally requires the node service lease's `connect_verified`
stream; origin metadata must never cause an independent dial. That producer and
the real process/profile owner are still missing.

## Lifetime rules

A caller owns a waiter. A retained worker owns its permit, target and actual
cleanup work. A timeout cannot release those resources or undo a delivered
mutation. Pending cancellation atomically prevents a later effect; once started,
loss is uncertain and the owner quarantines further operations. Page close keeps
the resident target until ACK; cancellation does not consume its only handle.

The CDP join and proxy join remain inside retained owners across cancelled close
waiters. Their completed failures cannot become an empty successful second close.
A transport close is distinct from process cleanup. Proc's `OwnedGroupExit` proves
leader reap plus the original accepted cleanup request/already-absent group;
it does not prove every descendant's observed OS death. Profile and slot release
still require the real qualified cleanup boundary.

Each bound has one named source owner: command/pending/event queues in CDP,
input transactions in `input_lane`, blocking work in `preparation`, connections
and headers in `proxy`, DNS answers in `network`, refs in `references`, traversal
work/depth in `snapshot`, and bundle entries/depth/actual hashed bytes in `launch`.
Protocol text/image/inventory bounds stay in `vessel-proto::browser`. Combining
these bounds into the real actor's total resource admission remains required.

## Verification and limits

Run narrow Rust checks on the approved build worker, at an immutable commit:
`cargo clippy -p vessel-browser --all-targets -- -D warnings` and
`cargo test --locked -p vessel-browser --lib`. The ordinary suite uses controlled
local peers and filesystem fixtures. Explicit ignored candidate fixtures require
the approved runtime host, service account and fixed artifact provenance; they
must not silently install a runtime or select another executable.

The exact exported `d37826ca` artifact passed the service-account bundle-read
fixture after the provisioning owner corrected two unreadable package lists.
The first PermissionDenied failure is retained. This fixture starts no browser;
it settles only the bundle's real account read/identity component. The later
input/delivery/close/forwarding changes have source-fast checks only and still
owe changed-source compilation and functional execution.

General qualification covers the launcher, pipe, process and profile lifecycle,
destination policy, IPv4/IPv6 TCP/UDP/ICE and QUIC/WebTransport controls, frame
decoding and input fencing. Preview has its own lease, revocation and profile
isolation gate. Agent discovery waits for every tool to qualify. Windows engine
qualification remains pending.

The independently authored script's document-envelope semantics also have a
source-only fixture: `node --test crates/vessel-browser/src/snapshot/document.test.mjs`
from the repository root. Its minimal DOM checks synchronous descriptor capture
and UTF-8 bounds. It does not emulate or qualify a browser, isolated world,
navigation, network policy or the Rust/CDP decoding path.
