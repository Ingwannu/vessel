# 0092: Shared keyboard predicates and source-owned command contributions

Status: proposed, F4 implementation in progress.

## Purpose and intent

Make keyboard hints, Settings, help and command-center actions agree, while
retained feature areas and plugins can safely replace their own registrations.
The existing palette and shortcut registry are the single entry points.

## Existing implementation and constraints

The native app already has a single keystroke interceptor, a pure shortcut
registry, preference-backed edits, a pure ranked palette and shared kit rows.
Its older defaults came from the previous web app. Paseo 0.15.4 is now the
behavioral reference; package owners provide typed feature actions and perform
their own current workspace/node/generation checks at dispatch.

## Alternatives considered

A second palette or listener would duplicate focus, IME, editing, ranking and
hint logic. A generic fuzzy-search/shortcut dependency would impose different
ranking and native editing behavior. String RPC dispatch in the registry would
lose the feature's typed contract and make stale plugin actions unsafe.

## Choice

Extend the existing pure modules. Binding predicates explicitly name platform,
host kind, focus scope, terminal/editing/command-center exclusions. Event
admission separately protects composing, character-preferred input, capturing,
inactive windows and top overlays; an override cannot weaken those protections.
A typed dispatcher orders enabled active handlers by priority then newest
registration. Source-owned contributions carry typed targets, nonempty choice
paths, ranks, visibility and translated presentation. Source/local ids remain
separate, and owner tokens protect replacements from stale cleanup. Matching
uses the existing shared text matcher, preserving stable registry order on ties.

## Why this choice

The existing assets already implement the right ownership layers: view for
rules, app for focus and feature routing, link for I/O, kit for drawing. Generic
contribution targets preserve each extension's concrete action type without a
parallel list of wire names. The UI still routes through each feature's existing
adapter. Missing feature adapters are reported rather than simulated.

## Advantages, costs and effects

Pure rules can be exercised without GPUI or a node. Atomic registration refusal
retains the previous valid source snapshot. Registry order and identity survive
query changes; generation overflow refuses replacement rather than reusing an
old token. No new dependency, RPC, protocol or persistence schema is needed.
Feature adapters and translated words still require integration and full UI
verification; pure-model checks alone do not establish working action sources.

See `docs/plans/wp-commands.md` for reference paths, explicit default migration,
platform differences, dependencies and package completion evidence.
