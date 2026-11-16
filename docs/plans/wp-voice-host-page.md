# Host Voice page: source contract and staged UI plan

Status: canonical values carrier53cf2e95 is scoped-reviewed and checked in
announced node head bc9cacc67. The app consumed only that selected carrier under
the A8 values exception, preserving its package base. Pure node-bound page
state and typed-fake tests are in preparation; the product settings page stays
unregistered until the real node settings field and adapter exist.

## Reference behavior

Paseo `packages/app/src/screens/settings/voice/voice-page.tsx`81–163 distinguishes
offline, unsupported and loading before drawing two sections: Dictation with
its switch/recognition site, and Voice mode with its switch/recognition and
synthesis sites. Unknown providers cannot open an editor. Each sheet opening
gets a fresh form identity (35–85); successful save refreshes the local-model
catalogue (55–63). Site summaries distinguish local model selection, endpoint
model/missing key, and unknown provider (229–241).

`speech-provider-form.ts`90–177 keeps the saved key private: a fresh key input
starts empty, blank means omission, and provider/model/language changes are
separate. Changed blank endpoint fields become resets. STT language survives a
provider change by an explicit patch when nonempty. Most environment-controlled
fields are read-only; API key/base URL are editable fallbacks (32–34/226–240).
Editing stops while saving and a failed save preserves the form (179–219).
The sheet closes after a successful save (`speech-provider-sheet.ts`90–103).

## Reuse and ownership

[Decision Log]
- Purpose: reproduce these page/form decisions without inventing a node call, mirroring DTOs, exposing a stored key or pretending an unavailable engine is installed.
- Existing assets: E3's settings sections/cards/rows/switch, G2's shared picker/dialog controls, G3's existing fields, S2's current menu/dialog leases, canonical DictationSettings/SetDictation and VoiceSettings/SetVoice, and existing protocol6 Admin `secret.set/list/delete` methods. No second settings or credential store is needed.
- Choice: three closed UI site identities, one node-bound read snapshot and fresh editor lease per opening. The fake adapter owns typed patch outcomes; pure draft validation uses the canonical overlay methods before any effect. UI callbacks verify node identity/editor generation and pending state. The eventual production adapter uses existing NodeSettings/SetNodeSettings and secret APIs only after their real fields/gates exist.
- Why: provider/site changes cannot silently overwrite a sibling site, retained callbacks cannot edit another host, and omission/reset behavior stays in the canonical owner instead of a copied form parser.
- Impact: use the shared responsive settings layout and controls, Fluent EN/KO text, explicit loading/offline/unsupported/read-failure states and preserved draft on failure. A failed read is not a default-Off configuration. No product registration or real I/O follows from the fake UI.

## Credential save boundary

The read DTO provides only KeyPresence. A new API-key input must use a shared
masked secret-field capability, start empty, never be prefilled from the node,
and never enter Debug, errors, logs, ordinary preferences or settings JSON.
Before implementation, verify the existing field capability with its owner;
do not fork a secret text renderer. Blank keeps the current reference.

Unlike Paseo's single feature patch, Vessel's credential owner and settings
writer are separate existing services. A supplied new key therefore needs a
typed two-step result: save through `secret.set` under an editor-owned fresh
reference, then apply the validated site patch referencing it. Reuse the same
reference on retry; do not overwrite a previously selected/shared key merely
to replace one site's credential. No new secret reader/writer is introduced.
A settings rejection after a successful key save is a partial result, not
success for the whole form. A lost reply is uncertain, not proof of rollback.
Do not automatically delete a key after an uncertain or failed settings write:
an applied reference or another writer may already use it. Static translated
messages must distinguish these results without appending foreign errors.

The fake must cover key-save refusal, settings refusal after key save, held or
lost replies, retry of the same editor reference, close/host change, and a late
completion that must not reopen or retarget an editor. Actual root writer,
credential ownership, permission and transport behavior remain separate checks.

## Explicit differences and remaining dependencies

- Vessel configures validated Endpoint sites and saved-secret references; raw credentials stay with the existing secret owner, not speech settings. No ambient environment fallback or fabricated env-override badge is added.
- The local model catalogue is selection metadata. LocalNotBuilt remains unavailable, with no automatic model download, fake progress or readiness inferred from a selected identifier. Model/runtime distribution and node model inventory/actions are later reviewed work.
- Provider reset and unknown-site behavior follow the checked DTOs. Unknown choices remain readable; an unrelated disable may preserve them, but editing that site requires an explicit known provider/model or reset.
- The fake UI uses typed data and effects only. No guessed `node.settings.voice`, new RPC, field, storage, actor/reload adapter or runtime is activated before its owning producer exists.


Shared key-field checkpoint: source inventory found no checked secret-entry
profile in the current kit. G3 confirmed its SearchField is not a substitute;
G2 owns the current TextField writer slot. The coordinator assigned G2 the
masked profile after its textarea slice, with ADR then V1 review. Keep key
entry plan-only until that checked hash arrives. Masking alone does not prove
clipboard, diagnostic, input-method or accessibility privacy.


## Page state implementation

The pure view::voice::settings owner uses the existing operation Ticket and
canonical read/write DTOs. Loading and failed reads have no default snapshot.
A switch changes only its own canonical switch patch, without round-tripping
provider/key fields. Saving excludes another edit; refusal preserves the last
confirmed snapshot and never claims the server rolled back. Offline/unsupported
or forbidden writes require a fresh read before editing resumes. A consumed
reply and a retained click from an older snapshot cannot apply twice or retarget
a different NodeId. Unknown providers remain readable but do not open a known
editor, and unknown switches are never interpreted as Off. The concrete fake
node in tests applies changes through the canonical overlay owners; it adds no
production store, guessed RPC, NodeSettings field or provider readiness. These
new UI-state regressions still await the next VM suite. Renderer, editor sheet
and credential-save controller remain separate outstanding page work.


## Unregistered renderer checkpoint

The staged area now composes the two sections and three detail rows with the
existing kit settings card, switch and custom-content row frame. Detail-row
summaries stay beside the title, with source hover/pressed tokens and the
chevron's one-token optical pull; an unknown provider keeps its summary and
loses its chevron/action. Offline and unsupported states use the shared warning
alert; read failure has a distinct message. Loading uses the existing spinner
and a labelled progress role. The reference ActivityIndicator large preset is
36 pixels (React Native Web0.21.x primary source,
https://github.com/necolas/react-native-web/blob/0.21.2/packages/react-native-web/src/exports/ActivityIndicator/index.js);
existing spacing tokens derive that size, without another spinner renderer.
All page messages and composed provider summaries are Fluent EN/KO sentences.
Callbacks carry the displayed node/ticket, not the currently selected host.
The fake UI opens only a recorded typed site action; an actual sheet is not
implemented yet. No secret field, settings route or Shell command is added.

New headless cases draw real rows in both schemes/languages at390/800/1280
widths and exercise a single pending switch plus blocked/fresh site actions.
Pure tests check full localized offline sentences and provider/key-presence
summaries. These new renderer tests are authored, not executed yet; no capture,
keyboard/OS assistive-technology or end-to-end user-flow acceptance is inferred.
The staged switches currently draw confirmed values without the optional
180ms transition counter; the final controller's animation and actual sheet,
refresh, permission/transport checks and credential partial-result flow are
still owed before product registration or parity approval.
