# ADR 0041: Account-free encrypted Web Push

Status: accepted, 2026-11-13.

## Context

An inbox cannot tell a person whose device is disconnected. Paseo sends
Expo notifications and stores expiring tokens; its policy picks a recently
active client and pushes only when nobody is present. GOAT's phone push
depends on Connect, which is disabled. Neither dependency is available to
Vessel: no owner-provided FCM/APNs project, account or key exists, and the
native phone host is paused.

## Options

1. Expo, FCM or APNs: needs accounts, credentials or a platform service.
2. A Vessel push relay: makes Vessel an operator of a new service and
   exposes attention routing metadata to it.
3. Standard Web Push (RFC 8030/8291/8292), with node-owned VAPID keys.
4. Only connected-device notifications: does not meet disconnected delivery.

## Decision

Option 3. A paired device registers its browser subscription's HTTPS
endpoint, P-256 public key and authentication secret over Vessel's existing
authenticated secure connection. Subscriptions belong to the authenticated
device, never to a device id supplied by another caller. Read scope is
sufficient; agent and delegated principals are refused. A device can replace
or remove only its own subscription. Revocation removes it atomically.
Subscriptions have a 48-hour lease, renewed by registration, to avoid keeping stale
capabilities forever. HTTP 404/410 removes an expired subscription.

The node generates a persistent VAPID private key in `push-vapid.key` in its
private data directory (0700 directory, 0600 regular file on Unix; the same
private-file checks as node credentials, no symlink following). On Windows
files inherit the data directory's ACL, as existing node credentials do;
Vessel does not set ACLs on a custom data path. That path's privacy must be
provided by the account's directory, and Windows ACL behavior is not yet
verified by this track. It is never on the
wire, never logged, never copied into a public artifact, and never replaced
silently on a read error. Only its public key is returned by `push.key`.
Losing that file requires devices to subscribe again.

The node uses RFC 8291 `aes128gcm` payload encryption and RFC 8292 VAPID
authentication. The payload contains only a version and a generic
"Something needs your attention" indication. No title, project, path,
prompt, command, question, file content, credential or session id enters an
OS or push notification. The app reconnects to fetch authorized inbox data.
The phone can use the same registration API later; this does not claim a
phone subscription UI or installed-phone delivery has been tested.

Node delivery follows Paseo's attention policy: an app-visible client focused
on this attention's session and active within 180 seconds suppresses push;
otherwise attention stays local rather than producing Web Push. With no
recently active client, push is eligible for subscribed devices. Merely keeping
a socket connected never suppresses push forever. Activity timestamps are
assigned by the node; a client cannot give a future timestamp. Failed/error
outcomes remain in the inbox but do not push, as in Paseo.
Connected clients apply the desktop/sound rules (whole-window focus silence
is a deliberate difference from Paseo, required by this track's brief): first snapshots do not
notify; a focused window is silent; only new attention transitions notify;
read/answered/deleted or archived entries never create new notifications.
Notification preferences and sound are applied by the client. Unlike Paseo's
selected-recipient local transport, Vessel's desktop gate is per window:
several unfocused connected windows may each notify according to their own
preferences. The node computes recent-recipient policy for the push decision
and does not emit a new unsolicited local-delivery frame. This preserves the
brief's unfocused-window rule and older clients; it does not claim global
single-recipient desktop delivery. The node
coalesces bursts, bounds concurrent HTTP sends and request time, does not
retry indefinitely, disables redirects and proxies, and permits only public
HTTPS endpoints (DNS resolved and pinned at delivery). A wire endpoint
cannot make the node connect to a private service. Tests use an explicitly
injected loopback transport, never a production endpoint exception. A restart
fixture reopens the same data directory, verifies the VAPID identity and
subscription survive, and observes the worker's exact newly admitted source:
old unread outcomes stay in the inbox without being replayed as pushes.

## What other parties learn

- A Vessel relay sees encrypted connection timing/size and node routing
  identifiers; it cannot read inbox data, subscription endpoints or keys.
- A push service sees the subscription endpoint, node source address, VAPID
  public identity, timing, TTL and ciphertext size. It can infer that
  something waits; it cannot decrypt what waits. Even the decrypted payload
  contains only the generic indication.
- A locked device shows only generic notification text. Opening it requires
  its existing paired credential, and its current scopes still apply.

## Why and consequences

Standards-based push is buildable without an operator account and keeps the
existing relay blind. The browser/platform still chooses a push service;
availability depends on that service and network, and delivery is best
effort. No promise of universal native-phone support is made. Encryption
and VAPID use established library primitives rather than a new cryptographic design;
`web-push-native` 0.5 (MIT OR Apache-2.0, default features disabled)
builds encrypted messages; RustCrypto `p256` signs the small typed RFC 8292
JWT envelope in `push/vapid.rs`,
while the existing bounded `reqwest`/rustls stack sends them. The initially
considered `web-push` 0.11 would pull ece's default OpenSSL backend, adding a
native crypto build dependency to every node target; it is not used. The
optional VAPID feature of web-push-native adds an all-algorithms JWT stack
(RSA, ML-DSA, unrelated ciphers) for one ES256 signature. It is disabled:
JSON/base64url are existing serializers, the claims are typed `aud/exp/sub`,
and `p256::ecdsa::SigningKey::try_sign` produces the fixed-width JOSE
signature. Tests verify the signature and every claim. No cipher, hash,
key derivation or curve arithmetic is implemented in Vessel.

Protocol 19's `push.key`, `push.subscribe`, `push.unsubscribe` and
`push.presence` are
additive and device-scoped; minimum protocol is unchanged. There is no
unsolicited new frame for an older client. Registering on an older node is
refused locally by method negotiation.

## References

- [RFC 8291: Message Encryption for Web Push](https://www.rfc-editor.org/rfc/rfc8291)
- [RFC 8292: VAPID](https://www.rfc-editor.org/rfc/rfc8292)
- [web-push-native 0.5 API](https://docs.rs/web-push-native/0.5.0/web_push_native/)
- Paseo: `packages/server/src/server/push/`,
  `packages/server/src/server/agent-attention-policy.ts`.

## Dependency licence inventory

The resolved `web-push-native` dependency closure below includes packages
already present in the workspace. New version entries are marked `new`.
Default features and the optional VAPID feature are disabled. Licences
come from the published package manifests at the locked versions.

| Crate | Version | Licence | Lock change |
| --- | --- | --- | --- |
| `aead` | 0.5.2 | MIT OR Apache-2.0 | existing |
| `aes` | 0.8.4 | MIT OR Apache-2.0 | new |
| `aes-gcm` | 0.10.3 | Apache-2.0 OR MIT | new |
| `base16ct` | 0.2.0 | Apache-2.0 OR MIT | new |
| `base64ct` | 1.8.3 | Apache-2.0 OR MIT | new |
| `block-buffer` | 0.10.4 | MIT OR Apache-2.0 | existing |
| `block-buffer` | 0.12.1 | MIT OR Apache-2.0 | existing |
| `bytes` | 1.12.1 | MIT | existing |
| `cfg-if` | 1.0.5 | MIT OR Apache-2.0 | existing |
| `cipher` | 0.4.4 | MIT OR Apache-2.0 | existing |
| `cmov` | 0.5.4 | Apache-2.0 OR MIT | existing |
| `const-oid` | 0.10.2 | Apache-2.0 OR MIT | existing |
| `const-oid` | 0.9.6 | Apache-2.0 OR MIT | new |
| `cpufeatures` | 0.2.17 | MIT OR Apache-2.0 | existing |
| `cpufeatures` | 0.3.1 | MIT OR Apache-2.0 | existing |
| `crypto-bigint` | 0.5.5 | Apache-2.0 OR MIT | new |
| `crypto-common` | 0.1.7 | MIT OR Apache-2.0 | existing |
| `crypto-common` | 0.2.2 | MIT OR Apache-2.0 | existing |
| `ctr` | 0.9.2 | MIT OR Apache-2.0 | new |
| `ctutils` | 0.4.8 | Apache-2.0 OR MIT | existing |
| `der` | 0.7.10 | Apache-2.0 OR MIT | new |
| `digest` | 0.10.7 | MIT OR Apache-2.0 | existing |
| `digest` | 0.11.3 | MIT OR Apache-2.0 | existing |
| `ecdsa` | 0.16.9 | Apache-2.0 OR MIT | new |
| `ece-native` | 0.5.0 | MIT OR Apache-2.0 | new |
| `elliptic-curve` | 0.13.8 | Apache-2.0 OR MIT | new |
| `ff` | 0.13.1 | MIT/Apache-2.0 | new |
| `generic-array` | 0.14.7 | MIT | existing |
| `getrandom` | 0.2.17 | MIT OR Apache-2.0 | existing |
| `ghash` | 0.5.1 | Apache-2.0 OR MIT | new |
| `group` | 0.13.0 | MIT/Apache-2.0 | new |
| `hkdf` | 0.12.4 | MIT OR Apache-2.0 | new |
| `hkdf` | 0.13.0 | MIT OR Apache-2.0 | new |
| `hmac` | 0.12.1 | MIT OR Apache-2.0 | new |
| `hmac` | 0.13.0 | MIT OR Apache-2.0 | existing |
| `http` | 1.5.0 | MIT OR Apache-2.0 | existing |
| `hybrid-array` | 0.4.15 | MIT OR Apache-2.0 | existing |
| `inout` | 0.1.4 | MIT OR Apache-2.0 | existing |
| `itoa` | 1.0.18 | MIT OR Apache-2.0 | existing |
| `libc` | 0.2.191 | MIT OR Apache-2.0 | existing |
| `opaque-debug` | 0.3.1 | MIT OR Apache-2.0 | existing |
| `p256` | 0.13.2 | Apache-2.0 OR MIT | new |
| `pem-rfc7468` | 0.7.0 | Apache-2.0 OR MIT | new |
| `pkcs8` | 0.10.2 | Apache-2.0 OR MIT | new |
| `polyval` | 0.6.2 | Apache-2.0 OR MIT | new |
| `primeorder` | 0.13.6 | Apache-2.0 OR MIT | new |
| `rand_core` | 0.6.4 | MIT OR Apache-2.0 | new |
| `rfc6979` | 0.4.0 | Apache-2.0 OR MIT | new |
| `sec1` | 0.7.3 | Apache-2.0 OR MIT | new |
| `sha2` | 0.10.9 | MIT OR Apache-2.0 | existing |
| `sha2` | 0.11.1 | MIT OR Apache-2.0 | existing |
| `signature` | 2.2.0 | Apache-2.0 OR MIT | new |
| `spki` | 0.7.3 | Apache-2.0 OR MIT | new |
| `subtle` | 2.6.1 | BSD-3-Clause | existing |
| `typenum` | 1.20.1 | MIT OR Apache-2.0 | existing |
| `universal-hash` | 0.5.1 | MIT OR Apache-2.0 | existing |
| `version_check` | 0.9.5 | MIT/Apache-2.0 | existing |
| `wasi` | 0.11.1+wasi-snapshot-preview1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | existing |
| `web-push-native` | 0.5.0 | MIT OR Apache-2.0 | new |
| `zeroize` | 1.9.1 | Apache-2.0 OR MIT | existing |

Primitive provenance: RFC 8291 ECDH uses RustCrypto `p256` /
`elliptic-curve`; `ece-native` performs HKDF-SHA-256 using `hkdf` /
`hmac` / `sha2`, and AES-128-GCM using `aes-gcm` / `aes` / `ghash` /
`polyval`. `rand_core` obtains operating-system randomness through
`getrandom`. RFC 8292 ES256 uses `p256` / `ecdsa`, with SHA-256 and
RFC 6979 deterministic nonces (`sha2`, `rfc6979`, `hmac`). HTTP TLS uses
the existing rustls/ring stack. No OpenSSL or native crypto build step is
introduced. The locked graph includes target-specific support crates;
a licence listing is not a claim of a third-party security audit.
