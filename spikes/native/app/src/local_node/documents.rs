//! Desktop registry traversal stays outside the reusable document control seam.

use crate::quit::{self, DocumentReadiness};
use gpui::App;

/// Run only after the initiating Shell/Window borrow ends. Final admission has
/// no subsequent await before `App::quit` or `updater.restart`.
pub(crate) fn admit(cx: &mut App) -> bool {
    admit_matching(cx, |shell, cx| quit::readiness(shell, cx))
}

pub(crate) fn admit_node(node: vessel_proto::NodeId, cx: &mut App) -> bool {
    admit_matching(cx, |shell, cx| quit::readiness_on(shell, node, cx))
}

fn admit_matching(
    cx: &mut App,
    readiness: impl Fn(&crate::shell::Shell, &App) -> DocumentReadiness,
) -> bool {
    let blocked = crate::desktop::shells(cx)
        .into_iter()
        .filter_map(|weak| weak.upgrade())
        .map(|shell| readiness(shell.read(cx), cx))
        .find(|readiness| *readiness != DocumentReadiness::Ready);
    let Some(blocked) = blocked else {
        return true;
    };
    crate::desktop::with_focused_shell(cx, |shell, _window, cx| {
        let _refused = quit::report(shell, blocked, cx);
    });
    false
}
