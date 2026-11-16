# Field control resolution over official package base13

[Decision Log]
- Purpose: resolve the official E2 root/presentation merge for the common input owner without restoring an old whole branch or creating another renderer.
- Existing assets: official2eb field split, FieldId, source/IME registry, FormMinimum and the immutable common2fd controls. All required display-map/set_auto_grow accessors already exist on the official engine; no vendored engine delta is needed.
- Choice: retain exactly the11 existing common control blobs as an additive base13 field-only leaf. All unchanged official style/frame/rename/select-all/search/editability/minimum/hooks/source tests remain their exact canonical blobs. E2 public element delegates the Composer arm to the sole composer_layout renderer; form and line rendering stay shared.
- Impact: preserve FormMinimum, ComposerLayout growth/font policy, SendOn::ButtonOnly preference protection, one field-local SubmitPress Rc initialized in both factories, one RAII scope around callbacks.submit, captured Plain Fn and unchanged Identified/FieldId handling. No editor Entity, event subscription or helper is duplicated.

This is source preparation for W6's existing common merge and ONE receiving check, not an independently executed producer. It has no App/view/link/root/Cargo/ADR0100/Markdown delta. It does not include K's later92ad Windowed delta; if that is separately adopted, preserve Windowed beside Plain/Identified and the same common dispatch rather than overwriting it with these blobs. Keep the consumer's existing registrations and boundaries. No positive delivery or runtime authority is introduced.
