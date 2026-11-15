# B2 scoped input adapter preparation

[Decision Log]
- Purpose and intent: make the existing composer presentation exhaustive over the canonical Workspace DraftKey without selecting another draft or creating another editor.
- Existing assets and constraints: W6 owns exact-key composer_field and composer_destination; W3 owns the retained field/mode and unavailable creation ingress. W12 owns resident presentation synchronization and admission. Canonical profile/ClientMessageId adoption remains held.
- Alternatives: map Workspace to global New, duplicate a field/hook/control, reuse another provider catalog, or infer send readiness from a mode getter.
- Choice: project mode from the exact W3 getter, share composer_element/layout and one hooked callback, validate the current destination on every key/paste/fullscreen action, and retain original owner callbacks. Readonly background rendering can read its exact field without granting action authority.
- Why: the same Entity must preserve text, selection and editing state; field access and typed display policy cannot mint root/config/send admission.
- Advantages, costs and effects: one existing input/render path handles both fonts and fullscreen. Missing keys refuse; Workspace send remains disabled, and its command list stays unavailable while no authoritative config snapshot exists. No provider list is borrowed from New/session, no synthetic Enter or second state store is introduced.

Source prerequisite chain is pinned on44b7b55f: B1 native6531db61a +98c83ce01 +bfc65599a, W3 native companion db5b4ac5f (equivalent02f120090), card18aefa314 and typing persistence78c2807c7 (equivalent5350ecd94). Root/Cargo/web are unchanged. These producer cuts are preparation only; their predecessor strict checks do not check this combination.

B2 owns only slash/input/client/admission, composer_shell presentation and the additive Surface mode projection here. W6 must delegate the workspace card's single input child to workspace_text; W3's terminal renderer must use that same input. Their trays, context state, creation state and callback ownership are not copied. Existing legacy paste callbacks remain for New/Session; Workspace uses the captured-key intake. File keys precede slash. Workspace slash completion explicitly does not use a catalog until its root/config contract exists.

All product adoption and fresh combined Cargo checks remain held until the announced checked profile/identity route is available. Then run one exact combined all-target App/View/Link/Kit check and the meaningful owned tests. The main44b source stays pinned for the producer cut; no new test-only resource attributes are removed. Legacy prebuilt screens, manually synchronized resident flags and the existing metadata target bridge do not prove a live lazy-body/visible-focus cutover or network authority.
