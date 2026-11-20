//! The list across nodes: sort, a failure that does not hide the others,
//! and a node that has not answered, which is not an empty list.

use vessel_proto::ErrorCode;
use vessel_proto::NodeId;
use vessel_proto::automation::{ScheduleInfo, ScheduleList};

use super::{
    SchedulePeer, ScheduleRow, SessionDirectory, actions, badge, bucket, hold_of, kind, meta,
    open_of, title,
};

/// How many schedules a full reply means. The node keeps at most this many
/// people's schedules (ADR 0033); a reply of this length says so.
pub const PEOPLE_LIST_CAP: usize = 256;

/// Why a node's list did not arrive. An older handshake is
/// [`NodeNotice::Older`], not a failure and not an empty list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadFault {
    /// No connection to that node.
    Unreachable,
    /// The node answered with this code.
    Remote(ErrorCode),
}

/// One node in the list's input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeReport {
    /// Handshake older than protocol 11.
    Older {
        /// Which node.
        node: NodeId,
    },
    /// The list has been asked and has not arrived.
    Waiting {
        /// Which node.
        node: NodeId,
    },
    /// The ask failed. Other nodes are still listed.
    Failed {
        /// Which node.
        node: NodeId,
        /// Why.
        fault: LoadFault,
    },
    /// A reply. A peer of [`SchedulePeer::Older`] is reported as older and
    /// the list is not shown as current.
    Listed {
        /// Which node.
        node: NodeId,
        /// How it is reached, and whether preview is offered.
        peer: SchedulePeer,
        /// Loaded or still pending. Not read by the badge.
        directory: SessionDirectory,
        /// The reply.
        list: ScheduleList,
    },
}

/// Something the list says besides a row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeNotice {
    /// Schedules are not on this machine.
    Older(NodeId),
    /// Not an empty list: the node has not answered.
    Waiting(NodeId),
    /// This node failed. Its rows are absent; other nodes remain.
    Failed {
        /// Which node.
        node: NodeId,
        /// Why.
        fault: LoadFault,
    },
    /// The reply had [`PEOPLE_LIST_CAP`] schedules.
    Capped(NodeId),
}

/// Rows in list order, and the notices beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePage {
    /// Newest created first. Tie-break is node id, then schedule id.
    pub rows: Vec<ScheduleRow>,
    /// Older, waiting, failed, and capped nodes.
    pub notices: Vec<NodeNotice>,
}

/// Assemble the page. `outstanding` is the schedules whose run request has
/// not been answered; run now stays hidden for those.
#[must_use]
pub fn page(nodes: &[NodeReport], outstanding: &[vessel_proto::ScheduleId]) -> SchedulePage {
    let mut rows = Vec::new();
    let mut notices = Vec::new();
    for node in nodes {
        match node {
            NodeReport::Older { node } => notices.push(NodeNotice::Older(*node)),
            NodeReport::Waiting { node } => notices.push(NodeNotice::Waiting(*node)),
            NodeReport::Failed { node, fault } => notices.push(NodeNotice::Failed {
                node: *node,
                fault: *fault,
            }),
            NodeReport::Listed {
                node,
                peer: SchedulePeer::Older,
                ..
            } => notices.push(NodeNotice::Older(*node)),
            NodeReport::Listed {
                node,
                peer,
                directory,
                list,
            } => {
                if list.schedules.len() == PEOPLE_LIST_CAP {
                    notices.push(NodeNotice::Capped(*node));
                }
                for info in &list.schedules {
                    rows.push(row_of(*node, peer, *directory, info, outstanding));
                }
            }
        }
    }
    rows.sort_by(|left, right| {
        right
            .info
            .created_at
            .cmp(&left.info.created_at)
            .then(left.node.cmp(&right.node))
            .then(left.info.id.cmp(&right.info.id))
    });
    SchedulePage { rows, notices }
}

fn row_of(
    node: NodeId,
    peer: &SchedulePeer,
    directory: SessionDirectory,
    info: &ScheduleInfo,
    outstanding: &[vessel_proto::ScheduleId],
) -> ScheduleRow {
    let actions = actions(info, peer, hold_of(info, outstanding));
    ScheduleRow {
        node,
        info: info.clone(),
        kind: kind(info),
        badge: badge(&info.state),
        bucket: bucket(&info.state),
        title: title(info),
        meta: meta(info),
        open: open_of(&actions),
        actions,
        directory,
    }
}
