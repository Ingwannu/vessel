//! The form's base choices: "Default branch" (sent as absent: the node's
//! project default, as `gh pr create` without `--base`), then the
//! repository's local branches but the one being proposed, the default
//! one first; and the base the form starts with.

use vessel_proto::workspace::{BranchList, BranchName, RefName};

use crate::workspace::git::{names_default, project_default};

/// One choice of the base select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseChoice {
    /// The branch; `None` is the node's default.
    pub branch: Option<BranchName>,
    /// Whether it is the project's default branch.
    pub default: bool,
}

/// The choices from `branches`, without `head` (the branch proposed).
#[must_use]
pub fn base_choices(branches: Option<&BranchList>, head: Option<&BranchName>) -> Vec<BaseChoice> {
    // The node marks `origin/main` when the remote's HEAD names it, else
    // the local branch (`git::project_default`).
    let default = branches.and_then(project_default);
    let mut listed: Vec<BaseChoice> = branches
        .map(|list| list.branches.as_slice())
        .unwrap_or_default()
        .iter()
        .filter(|branch| branch.remote.is_none())
        .filter_map(|branch| {
            let name: BranchName = branch.name.as_str().parse().ok()?;
            let is_default = default.is_some_and(|default| names_default(default, name.as_str()));
            (Some(&name) != head).then_some(BaseChoice {
                branch: Some(name),
                default: is_default,
            })
        })
        .collect();
    // The default first; the rest keep the node's order (newest commit
    // first, as `git.branches` lists them).
    listed.sort_by_key(|choice| !choice.default);
    let mut choices = vec![BaseChoice {
        branch: None,
        default: false,
    }];
    choices.extend(listed);
    choices
}

/// The base the form starts with, from the one the node measures the
/// folder against (`GitStatus.base`), read with the repository's
/// `branches`.
///
/// [Decision Log]
/// - Purpose: start the form on the base the branch was made from,
///   spelled as the forge takes it.
/// - Constraint: the node measures a project's folder against its
///   default as it marks it (`origin/main` when `origin/HEAD` names it),
///   and sends `pr.open`'s base to the forge as written; a forge has no
///   branch called `origin/main` (the node strips the prefix only for its
///   own default, `workspace/forge.rs` `base_branch`).
/// - Alternatives: the name as the status gives it (a real forge refuses
///   it); always the node's default (loses a worktree's own base).
/// - Choice: absent (the node's default, `gh pr create` without
///   `--base`) when it is the project's default in either spelling or
///   before the list is read; a remote-tracking branch without its
///   remote's name; any other local branch as it is.
/// - Cost: the start is read again on each frame, so a list that arrives
///   after the form opened corrects it; a person's own choice is kept by
///   the caller.
#[must_use]
pub fn start_base(base: Option<&RefName>, branches: Option<&BranchList>) -> Option<BranchName> {
    let (base, list) = (base?, branches?);
    let local = list
        .branches
        .iter()
        .find(|branch| branch.name == *base)
        .and_then(|branch| branch.remote.as_ref())
        .and_then(|remote| {
            base.as_str()
                .strip_prefix(remote.as_str())
                .and_then(|rest| rest.strip_prefix('/'))
        })
        .unwrap_or(base.as_str());
    let is_default = project_default(list)
        .is_some_and(|default| names_default(default, local) || default.as_str() == base.as_str());
    if is_default {
        return None;
    }
    local.parse().ok()
}

#[cfg(test)]
mod tests {
    //! The node's default comes first, as "Default branch"; the proposed
    //! branch and remote-tracking ones are never a base.

    use chrono::{TimeZone as _, Utc};
    use vessel_proto::workspace::{Branch, BranchList};

    use super::{base_choices, start_base};

    fn branch(name: &str, remote: bool, default: bool) -> Branch {
        Branch {
            name: name.parse().unwrap(),
            remote: remote.then(|| "origin".parse().unwrap()),
            current: false,
            default,
            head: "0123456789abcdef0123456789abcdef01234567".parse().unwrap(),
            time: Utc.with_ymd_and_hms(2026, 11, 14, 9, 0, 0).unwrap(),
            upstream: None,
            worktree: None,
        }
    }

    #[test]
    fn default_branch_leads_then_the_project_default_then_the_rest() {
        let list = BranchList {
            branches: vec![
                branch("feature/widget", false, false),
                branch("develop", false, false),
                branch("origin/main", true, true),
                branch("main", false, false),
            ],
            more: false,
        };
        let head = "feature/widget".parse().unwrap();
        let names: Vec<Option<String>> = base_choices(Some(&list), Some(&head))
            .into_iter()
            .map(|choice| choice.branch.map(|name| name.to_string()))
            .collect();
        assert_eq!(
            names,
            vec![None, Some("main".to_owned()), Some("develop".to_owned())]
        );
    }

    #[test]
    fn before_the_list_is_read_only_the_node_s_default_is_offered() {
        assert_eq!(base_choices(None, None).len(), 1);
    }

    #[test]
    fn the_start_is_the_node_s_default_for_the_project_default_in_either_spelling() {
        let list = BranchList {
            branches: vec![
                branch("origin/main", true, true),
                branch("main", false, false),
                branch("develop", false, false),
                branch("origin/release-2", true, false),
            ],
            more: false,
        };
        let start = |name: &str| start_base(Some(&name.parse().unwrap()), Some(&list));
        assert_eq!(start("origin/main"), None);
        assert_eq!(start("main"), None);
        assert_eq!(start("develop").unwrap().as_str(), "develop");
        assert_eq!(start("origin/release-2").unwrap().as_str(), "release-2");
        assert_eq!(
            start_base(Some(&"develop".parse().unwrap()), None),
            None,
            "before the list is read"
        );
        assert_eq!(start_base(None, Some(&list)), None);
    }
}
