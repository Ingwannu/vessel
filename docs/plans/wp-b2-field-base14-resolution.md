# B2 base14 field conflict resolution

This is a source-only two-file conflict assembly for the already-owned common
composer input union, based on official package base14.1. It is not a standalone
B2 feature or an independently compiled field producer: the consumer retains
its prior composer_layout, submit_press, send, boxed Plain callbacks and tests.

The root keeps incoming composition/accessibility registration, FormText and
FormTextareaPresentation fields/defaults, and the existing shared FormMinimum
shell. Both composer row and submit-press Rc slots remain on the same field.
The existing line factory retains its one listen call, original submit Rc and
composition registration; no second Entity or subscription is introduced.
Presentation keeps incoming form text/chrome refinement while its composer arm
still delegates to the one owned composer_layout renderer. Form Enter bubble
handling remains unchanged. Import resolution removes constants that the
incoming form presentation now consumes inside its own module.

Only field.rs and field/presentation.rs are supplied as resolved blobs. The
receiving owner resolves other modules/constructor defaults and runs the one
combined common strict/runtime check. No readonly/IME/composition/accessibility
or application acceptance is claimed from this source resolution or rustfmt.
