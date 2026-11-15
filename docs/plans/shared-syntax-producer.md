# Shared syntax producer

The single palette/role table feeds both public CodeStyles and legacy
CodeTokens::of; the old syntax table is removed, while old error/diff tints
stay unchanged. SyntaxKind aliases the shared semantic vocabulary, preserving
the seven original names. app::syntax_kind::of(TokenKind) is the unchanged
seven-arm lexer bridge moved into one place. There is no new lexer, renderer,
font, field, preview or preference.

API: kit::code_editor::CodeStyles::of(Scheme).highlight(SyntaxKind),
kit::syntax::{SyntaxPalette,SyntaxStyle,CodeRun}, set_syntax_palette.
Keep newer CodeEditor hooks and the W13/G1 syntax_for factory on union;
never copy a whole older root or duplicate mapping/colour tables.

Evidence scope: producer e8051e72e exact Kit lib228/0/0, run
themes-narrow-20261115T184906Z-e8051e72. Prior2c170f575 whole strict/build
passed and App99/0/2, but its package tests were red; the corrected full F5
gate and UI review remain pending. This portable assembly has not had an
independent native run. Consumer must check its own union; no standalone
PASS, V1/full F5/UI/OS/hardware acceptance is claimed.
