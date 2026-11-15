# Host Voice page: source contract and staged UI plan

Status: planning only. The canonical values carrier must be reviewed, checked
and announced before the fake UI consumes it. The product settings page stays
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
