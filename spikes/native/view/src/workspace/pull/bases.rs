//! The form's base choices: "Default branch" (sent as absent: the node's
//! project default, as `gh pr create` without `--base`), then the
//! repository's local branches but the one being proposed, the default
//! one first.

use vessel_proto::workspace::{BranchList, BranchName};

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

#[cfg(test)]
mod tests {
    //! The node's default comes first, as "Default branch"; the proposed
    //! branch and remote-tracking ones are never a base.

    use chrono::{TimeZone as _, Utc};
    use vessel_proto::workspace::{Branch, BranchList};

    use super::base_choices;

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
}
