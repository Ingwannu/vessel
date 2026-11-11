# Web UI design

`DESIGN.md` is the contract for `vessel ui`'s page (ADR 0017): principles,
information architecture, tokens with their measured contrast, components,
interactions, accessibility.

There is one design system, and it lives in the app (ADR 0018):

- **Tokens:** `web/src/ui/tokens.css`. White and black: a white canvas,
  near-black ink, the ink as the accent, hue only for state and syntax.
  Dark is the same system inverted.
- **Components:** `web/src/ui/app.css`, tokens only. A literal colour
  there is a defect, except the pairing QR code.
- **Pictures:** the app's own screens, captured by Playwright against real
  nodes with the fake harnesses: `npm run screens` in `web/` writes every
  main screen, in both themes, to `web/artifacts/screens/` (untracked).
  CI uploads them.

## History

- **Milestone 7** built the first version from a design draft kept in this
  directory: a dark, blue-accented token file, a stylesheet and seven
  static HTML mockups. The draft settled the design before any code
  existed.
- **Milestone 8** (ADR 0018) made the UI white and black, turned Sessions
  into a board, and removed the draft's token file, stylesheet and
  mockups: kept beside the app's they were a second design that drifted.
  They are in the history at `400baae` and before.

## Changing the design

1. Change the token or the component in `web/src/ui/`.
2. Update `DESIGN.md` in the same change. A changed colour re-measures its
   contrast pairs (section 4.3).
3. Run `npm run lint` (it includes the font rule), `npm run screens`, and
   look at the screens in both themes.
