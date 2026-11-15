# Shared speech audio codecs

[Decision Log]
- Purpose: reuse the existing tested PCM16 conversion and WAV format for
  dictation and spoken conversation without sharing speech policy.
- Existing assets and constraints: the dictation implementation already has an
  integer phase/carry converter and in-memory WAV encoder. Voice capture has a
  distinct validated PCM packet; converting it to a dictation packet would copy
  or couple domains. Provider metadata and request policies are separate.
- Alternatives: duplicated codecs, a general audio framework, or reusing a whole
  dictation segment/provider. Each adds drift, dependencies or unrelated policy.
- Choice: modify/reuse the numeric algorithm in one node module, with borrowed
  aligned input, a typed existing rate, a redacted provider PCM result and WAV
  view. No dependency is added. Successful samples and framing are preserved.
- Why: both consumers can append the same format directly into their own bounded,
  redacted bodies. Matching-rate bypass borrows samples and conversion carries
  only one prior sample and phase.
- Effects: checked count/allocation refusals are typed; callers still own capture
  nonemptiness and recording budgets. The module initially compiles only in tests
  until a real consumer removes staging. No provider, method or number is added.

## Reference evidence

Paseo server/agent/pcm16-resampler.ts31–41 and63–67 retain carry; its interpolation
uses Math.floor/Math.round. server/speech/providers/openai/stt.ts72 and80–104 select
the provider rate and encode PCM16 mono WAV. The earlier ADR0067 comparison fixture
pins counts at six rates and the limited one-LSB/one-sample difference on signed
extremes and zero crossing. This extraction preserves that stated difference;
no general recognition or arbitrary-audio equivalence is claimed.

## Validation and resume

Narrow strict node and proto Clippy passed for the codec addition. The merged
prerequisite snapshot also passed node all-target Clippy and all eight codec
cases together with the two connection-lifetime cases. These are staged-source
checks, not a product or recognition gate.
Cases cover six-rate chunk invariance/counts/signed fixtures, borrowed bypass,
WAV metadata and body prefix, alignment, empty codec output, RIFF count refusal
and allocation refusal before mutation. No OS, audio device, provider, credential
or recognition test has run here. Next: review this pure dependency, then each
consumer replaces its existing codec call sites without moving its policy.

## Shared settings decoder visibility

Spoken conversation also needs omission/null/set decoding and the redacted
selected-secret-name boundary. Reuse the existing visitor through a crate-only
deserialize_change alias and widen selected_name only to its owning proto crate.
Existing dictation fields use that alias too; no parser, rule, representation,
lookup or storage changes. The existing patch/reset/credential-refusal cases
remain unchanged. Narrow checks for this visibility-only follow-up are pending.
