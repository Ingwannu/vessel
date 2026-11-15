# Voice glyphs

Lucide1.50.0, immutable commit
[9d9418a25d3de4f6511a67848d4e7c76a897ba8e](https://github.com/lucide-icons/lucide/tree/9d9418a25d3de4f6511a67848d4e7c76a897ba8e/icons):
mic.svg, mic-off.svg and square.svg. The adjacent LICENSE-lucide.txt contains
the upstream ISC and applicable Feather MIT notice; redistribute it with the
native application's third-party notices.

Changes: stroke2 to2.5 to match the pinned Paseo realtime overlay; currentColor
to black for GPUI's existing shared monochrome asset mask. Square uses intrinsic
black fill (the source overlay's Square fill=white, recolored by the shared
button ink); this does not change the existing outline Stop vector. G2 owns
button color,20px sizing,44px geometry and switching/spinner behavior. W19 owns
only these append-only glyph registrations and voice_meter.

V1 reserved Mic/MicOff/SquareFilled for W19; timeline MicVocal remains W10's.
No new icon dependency or runtime network lookup is added. This source/readme
is provenance, not final binary-distribution or inspected UI proof.
