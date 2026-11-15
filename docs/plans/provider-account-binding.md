# Provider account binding increment

Reuse the manifest for legacy name admission; do not create another reserved
name registry or reinterpret saved Added records. The pure manifest regression
and real-node add/refusal/no-file-mutation regression are drafted. No real
provider is launched, no credentials are used, and execution is still owed.

The actor-owned binding producer is a separate prerequisite, agreed with the
lifecycle owner: actual SessionId/run ProcessLifetime, HarnessRef/resolved
profile and immutable launch association are captured together; only an explicit
provider launch binding may associate ProviderId. Options fence before/after
same_run checks remain with that actor. No ready capture head exists yet.

The public snapshot must retain unknown when that attestation/sign-in observation
is absent or ambiguous. Merely matching family/ID, seeing credential files or
finishing a catalogue handshake is insufficient. No speculative binding fields
are enabled by this preparation; G3 keeps modern provider and legacy account
parts separate until a real producer can populate the typed join.

## Shared values checkpoint, not a snapshot join

The proposed shapes were confirmed with the lifecycle owner before implementation:
ProviderLaunchAssociation is non-serde explicit ProviderId/HarnessRef/ProfileName
data, not authority. ProviderBinding is Unobserved or Bound with the actual
resolved harness/profile and ProviderSignIn. Sign-in is Unknown or an Observed
closed verdict with the actual observation timestamp. Debug of the internal
selection is redacted; the reader carries no env/path/account/help command.

Two invariants govern a future producer: profile must be the actual resolved
Profile.name(), never SessionInfo.profile=None filled by a guessed name; and
observation evidence must be bound to the same actual association/run, not merely
a recent timestamp. The lifecycle actor retains that proof and performs the
before/after options fence and same_run validation. The value constructor has
no default mapping and cannot itself establish attestation. Missing or ambiguous
capture stays Unobserved; an unknown sign-in verdict stays Unknown.

These staged values introduce no snapshot field, request acceptance, method row,
protocol assignment, actor capture implementation or account join. Tests exercise
value retention/redacted Debug and the explicit known/unknown codec only; execution
and compilation are owed. Wire integration and older-peer masking must be added
with a real producer before G3 can merge its two UI parts.
