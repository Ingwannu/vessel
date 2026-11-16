# ADR0100 causal baseline on the product Kit graph

This immutable baseline tree registers the three original baseline assertions
as Kit-level tests. The existing product GPUI and native lock remain unchanged;
there is no upstream dev dependency graph, added dependency or new manifest.
The fixed test command enables the existing gpui-base/test-support feature.

HTML rendered/copy/highlight/reveal/invalid UTF-8/clear assertions and the
actual-drawn current-ink/font/clear-restoration assertions stay unchanged.
Only setup imports/init point at the existing base dependency. The baseline
uses only its existing public APIs; the candidate adds only its opt-in and
foreground builders in the two original setup helpers.

The approved identical change12 observer reads actual retained runs after the
same view is drawn, returning a copy rather than a mutable handle. This is the
observation used by the ink/font/clear assertions, not a stored acceptance flag
or a second renderer. The original private test module remains in the engine
source but is not claimed to execute through a dependency's cfg(test).

No causal run has executed. Baseline is expected to fail at real HTML
Unsupported and missing drawn ink; those are authored expectations only until
the tool-owned fixed Kit recipe records the actual failures. Candidate and
receiving Kit qualification remain owed. Source direction is not execution.
