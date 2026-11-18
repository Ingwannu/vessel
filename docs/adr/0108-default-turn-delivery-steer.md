# ADR 0108: The page's default turn delivery is Steer; stored choices are kept

Status: accepted (number assigned by the coordinator on 2026-11-17 for TRACKER 793); impl U.
Amends ADR 0028 (the bridge keeps the page's preferences) for one default.

## Context

A message sent while a turn runs is delivered one of three ways (protocol 10): queued for the turn's end, steered
into the running turn where the agent can, or interrupting the turn. The page's "Default send" (Settings > General)
is the person's choice, kept in the bridge preference document as `composer.delivery`.

- The bridge has defaulted to Queue since 5a6de668f (2026-11-12): `TurnDelivery` in vessel-web
  `wire/prefs.rs` has `#[default]` on `Queue`, and the page's `DEFAULT_PREFERENCES` says `"queue"`.
- Paseo defaults to Steer (`hooks/use-settings/storage.ts`: `sendBehavior: "steer"`), and the owner chose Paseo's
  default for Vessel (word 45). The native app already defaults to Steer (`view/src/prefs/mod.rs`).
- The keeper writes only fields a person set (`keep/prefs_doc`: each field is skipped while absent, since its
  first commit 78adb01e5, before delivery existed). The page writes `delivery` only from the Settings choice
  (`features/settings/general.tsx`). So a stored document carries `delivery` exactly when someone chose it.

## Decision

[Decision Log]
- Purpose: the page and the bridge default to what Paseo and the native app do, without overriding anyone's
  choice.
- Existing assets: the sparse keeper, the page's `sendPlan` (data/delivery.ts), the native `TurnDelivery`.
- Alternatives:
  (a) keep Queue, and document page and native as different (the owner's word 45 says otherwise);
  (b) Paseo's way, a migration that rewrites stored values of the old default (`migrations.ts` turns
      `"interrupt"` into `"steer"`): Paseo had to, because its store wrote the default; ours does not, and a
      rewrite would flip a Queue someone chose;
  (c) change the default in the bridge only, leaving the page's local defaults at Queue until the bridge pushes.
- Choice: `#[default]` moves to `Steer` in the bridge and `"steer"` in the page's `DEFAULT_PREFERENCES`, together.
  - A document without `composer.delivery` (or with one this build cannot read, the keeper's lenient rule)
    reads as Steer.
  - A document with an explicit `"queue"`, `"steer"` or `"interrupt"` keeps it. Nothing is rewritten or
    migrated, and the absent field stays absent.
- Why: it is the one change that makes every unset document read like Paseo and native while leaving every set
  one untouched; (b) cannot tell a chosen Queue from a default here and is not needed; (c) would show Queue for
  a moment before the bridge's first push.
- Costs and effects:
  - People who never chose see Send steer during a turn where the agent can. Where it cannot, the page keeps
    its stated fallback (data/delivery.ts: the message queues and the button says why). The native app
    replaces the turn there instead, as Paseo does; that difference is not part of this decision and is
    reported to the coordinator separately.
  - Tests and e2e that relied on the default now choose Queue explicitly where they test the queue.
  - The native test's copy of the page fixture loses its one stated difference once this reaches the app line.

## Consequences

- vessel-web: `TurnDelivery::default()` is Steer; the defaults contract writes `"delivery":"steer"`; keeper tests
  state that an absent field reads as Steer and an explicit Queue stays Queue.
- Page: `DEFAULT_PREFERENCES.composer.delivery` is `"steer"`; the conversation e2e sets Queue in Settings for the
  queue cases and puts the default back after.
