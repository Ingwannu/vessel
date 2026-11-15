# Assistant file-reference values

[Decision Log]
- Purpose and intent: share the composer file-token parser with timeline and file Markdown hosts without another recognizer or filesystem access.
- Existing assets and constraints: the composer already owns these pure values; Files owns authorized root lookup, navigation and read acquisition. The existing matcher owns JavaScript whitespace.
- Alternatives: duplicate the parser in each host, infer a host root from cwd, or import the full composer/runtime branch.
- Choice: export only view::composer_shell::links and widen the existing matcher whitespace helper to crate visibility without changing its body. No dependency is added.
- Why: authored hrefs decode percent bytes while inferred tokens retain them, and source byte ranges must remain stable for selection/find. Classification cannot grant file authority.
- Advantages, costs and effects: the same validated values are reusable by both owners; acquisition, suffix lookup, opener, Markdown callbacks and native provider behavior remain separate.

Reference: Paseo0.15.4 packages/app/src/assistant-file-links/parse.ts:58-68,106-119 and resolver.ts; components/markdown/source.ts:12-55. Recognized ranges address unchanged UTF8 source. Explicit href and fenced-code handling belongs to the existing Markdown host.

API: classify(value, Origin::{Href,Inferred}, Workspace::{Known,Unknown}) returns Link::{External(HttpUrl),File(FileTarget),Lookup(FileTarget)}. FileTarget has a PathToken and optional one-based inclusive Lines with NonZeroU32 endpoints. parse_file parses values only. recognize exports typed source-byte tokens for the same Markdown engine recognizer adapter. An absolute or home-relative classification still requires Files authorization; no arbitrary host read is enabled.

The source blobs and tests are identical to the composer slice executed in run wp-composer-shell-view-tests-20261115T135650Z-a809d84c (30/0 across all composer models, including these parser cases). That prior-source test run does not prove this new minimal package compiles independently. Exact70ee53e9 passed view all-target strict Clippy in run `wp-composer-link-values-20261115T144124Z-70ee53e9` (rc0, cargo39s). Its prior-source30-test execution remains separate; this values leaf is not a file opener or host authority. No Kit/App/transport/authority/capture approval is claimed.
