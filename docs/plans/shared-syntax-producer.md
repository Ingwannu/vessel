# Shared syntax producer

One role/palette table feeds public CodeStyles and legacy CodeTokens::of;
no second syntax colour table remains. Error/diff constants stay unchanged.
SyntaxKind aliases the shared vocabulary and preserves the seven names.
app::syntax_kind::of(TokenKind) is the single existing seven-arm mapping,
with no IO/new lexer or renderer. No field/font/preview/preferences travel.

API: kit::code_editor::CodeStyles::of(Scheme).highlight(SyntaxKind),
kit::syntax::{SyntaxPalette,SyntaxStyle,CodeRun}, kit::set_syntax_palette.
Preserve newer CodeEditor hooks and G1/W13 syntax_for factory on union;
never copy an old root or duplicate the mapping/colour table.

Scoped evidence: source c30229ff1 Kit/View/App all-target strict0,
themes-narrow-20261116T003505Z-c30229ff. Product code previously passed
Kit228/0/0 at e805; the test helper split retains all assertion expressions
and input/park calls (only the wildcard is made exhaustive). Full F5 gate,
UI and V1 approval are pending. This portable assembly has no independent
run: receiving consumers must check their union and preserve newer seams.
The earlier base9 39b47/a821 exports are superseded, not silently certified.
