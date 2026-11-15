# F1 menu dismissal and caller focus

[Decision Log]
- Purpose: fix the observed root-F5 routing loss after menu dismissal.
- Existing constraint: the shared published renderer/constructors stay; some
  callers close state outside an input callback and have no Window.
- Alternatives: refocus the whole app/window, an area keyboard workaround, or
  keeping a strong handle to a caller that may have been removed.
- Choice: a weak return target captured only at the closed→open boundary;
  pointer/keyboard/outside dismissal restores it before the chosen callback.
  KeepOpen keeps menu focus, and reopening an already open menu keeps its caller.
- Why and cost: normal key routing resumes without capturing global shortcuts
  or retaining removed views. A newer field/dialog's focus wins. State-only
  close remains source-compatible; callers with a Window can dismiss/restore.

The existing menu_right/TopRight, measuredEnd placement, QUICK surface arrival,
fill_in highlight, widths, colours and selection guards are unchanged. Both
pointer and keyboard still use the same selected-index/availability policy;
Pending/Success/disabled items do not activate. The action callback runs after
restoration, so a callback opening another dialog may deliberately take focus.
Outside-click order cannot override a field that has already taken focus.

Regression uses a real GPUI window and menu renderer: the root F5 callback is
observed before opening and after Enter, Escape, row click and outside click.
Separate cases cover KeepOpen/reopen, a newer focus owner and a removed return
handle. These are pending native tests, not an assertion that an X11/macOS/
Windows first-use capture or OS keyboard integration has passed. The old failed
Linux root-F5 observations remain retained, and the final F1 package must rerun
that actual input evidence. No area-local focus workaround was introduced.
