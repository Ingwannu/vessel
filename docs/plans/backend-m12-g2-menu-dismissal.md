# Shared menu dismissal policy

[Decision Log]
- Purpose: direct Git writes keep their pending feedback in the open menu.
- Constraint: existing pointer/Enter handlers closed before invoking callers.
- Alternatives: an area closes/reopens its popup, or every caller changes.
- Choice: per-item Dismissal::Close (default) / KeepOpen and with_dismissal.
  Pointer and Enter share the same guarded activation/close boundary.
- Why: Paseo actions-split-button.tsx passes closeOnSelect=false for direct
  overflow writes. Modal openers and schedules retain default dismissal.
- Cost: one typed item property; no extra renderer, success/error policy or
  automatic retry. Caller/view owns pending/result state and its words.

Pending/disabled items remain unselectable even under a stale keyboard
highlight. KeepOpen retains focus/highlight so the caller's updated item
can show its spinner; Escape and outside dismissal are unchanged. Tests
exercise actual MenuState open/closed state through both activation paths,
legacy default, pending/disabled refusal, stale indices and explicit close.
Native pointer/Enter pending/error evidence belongs to shared probes and
Git first use; unit/source checks alone do not certify the Git surface.
