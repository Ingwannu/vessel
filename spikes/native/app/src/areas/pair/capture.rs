//! A capture fixture, compiled only into Linux tests (never the shipped app).
//!
//! [Decision Log]
//! - Purpose: check the new browser-unsupported sentence in the real renderer.
//! - Constraint: a native link cannot emit that browser-only result. A native
//!   capture proves the words and layout, not a running browser transport.
//! - Choice: an explicitly ignored display test opens the ordinary shell and
//!   uses its real offer reader before supplying that one typed outcome.
//! - Why: no override, fake-event option or preview branch enters an installable
//!   app; the other failure outcomes are captured through their real paths.
//! - Cost: this visual fixture needs a private display, config and fleet offer.

use vessel_app_kit::ui::{Context, Window};
use vessel_app_view::pairing::PairProblem;

use crate::{routes::Route, shell::Shell};

pub(crate) fn browser_unsupported(
    shell: &mut Shell,
    pasted: &str,
    window: &mut Window,
    cx: &mut Context<'_, Shell>,
) {
    // An initial empty Fleet event must not clear this already-visible fixture.
    shell.loaded = true;
    shell.go(Route::Pair, window, cx);
    shell.read_offer(pasted, window, cx);
    assert!(
        shell.areas.pair.offer.is_some(),
        "the private offer must decode"
    );
    shell.pair_failed(PairProblem::Browser);
}
