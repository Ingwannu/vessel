# Borrowed tool-detail body prerequisite

Status: native source preparation only. This delta is based on official app
package base8 91407173. It does not compile standalone until W23's canonical
body-values prerequisite is reviewed, checked and integrator-adopted. Proposed
body-values leaf d596635d7dbe0b5f690dac6d8031fcd147907f04 is not assumed ready.

[Decision Log]
- Purpose: A8 draws a reported proposed ToolCallDetail without constructing a
  ToolView, Entry, managed session identity or another tool renderer.
- Existing assets/constraints: one kernel uses existing output/diff/prose/skeleton/
  alert controls. Existing Entry Paint is projected into four borrowed drawing
  inputs. Data and presentation never grant execution or approval authority.
- Alternatives: copying A7's full timeline state imports unrelated behavior;
  duplicating bodies lets syntax, error, range and copy handling drift.
- Choice: ONE tool_body implementation, with the same public tool_detail facade
  as A7. The facade accepts &Paint, &ElementId, Copy BorrowedDetail, Window, App.
  Shell contexts coerce to App. DetailContentState::{Loading,Ready} describes
  content availability only; A8's outer request may remain Pending.
- Reason: the body reads catalogs/words/tokens/secondary ink only; identity,
  disclosure, actions, I/O and protocol fields remain with the caller.
- Effects/tradeoffs: both new app modules are literal cfg(test) until the actual
  A8 caller lands. This is an explicitly staged prerequisite, not a shipped rich
  permission card or node activation. Before enabling it, the real caller and
  legacy computed-diff delegation must land in the same product union so there
  is one production diff body. No fake ToolView or synthetic receipt is supplied.

The included bounded diff/word/section models and kit output decorations are
native presentation prerequisites only. Existing Line/Lines/output APIs remain
valid. Shared range validation is moved once, not copied. Source files for the
kernel, model, parser, kit and EN/KO data match the A7 authored source; the facade
is a subset with no header/provenance/resident dependencies. Diff model adaptation
updates only the legacy side/optional-number match, without a new controller.

Validation owed: exact strict View/Kit/App, unit and actual headless borrowed API
draw tests against the approved canonical values. No whole gate/capture/V4 pass
is claimed; A7's final package keeps its single full gate/review. Prior A7 evidence
(1153 view; 271 app/kit) does not prove this isolated source. The new staged fixture
draws proposed material without ToolView/log/session objects. W4 owns actual
permission/question/controller integration and its independent verification.

Source amendment: facade types are exercised through their exported path. The
same headless fixture also changes proposed Ready content into an empty Loading
body and draws it, retaining one stable structural identity. No execution or
approval outcome is fabricated. Output range assertions use first()/from_ref;
the earlier A7 actual test pass preceded this isolated draw fixture.

Dependency audit: the existing empty-JSON predicate now lives in tool_sections,
with A7's header model importing it there. The body does not import tool_display
or its source/provenance graph; no second predicate or decoder is introduced.
The moved computed-diff implementation imports FluentBuilder at its new call
site; pinned GPUI defines conditional building on that separate trait. This
static import audit is source preparation, not a compiler or draw-test pass.


Preparation-only canonical dependency intake: full62d8743c8..d596 body values
are present after announced node gate50c (9c150c6e, all18steps0, Rust2549/0/12,
including body tests). This is vocabulary only. Coordinator permits preparation
here but requires merging announced green app base9 before the actual handoff;
any already-contained isolated pick must then disappear from the final delta.
No rich fields/events/status/RPC/receipt/runtime activation is introduced.

Dependency closure includes the native workspace declaration for the already
locked similar2.7.0 and its actual About inventory entry in manifest order. The
initial source-only cut had only view's workspace inheritance, not that required
parent declaration; static manifest intake found and corrected it before any
isolated compile was submitted. SPDX Apache-2.0 is verified from the locked
registry package. This reuses the existing text-editing category, with no new
wording table or dependency version. Exact standalone native checks remain owed.
