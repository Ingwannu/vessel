//! Only explicit profile references become configured model use.
use super::{
    User,
    page::{Listed, Snapshot},
};
use vessel_proto::ModelRoute;

/// The three facts the node can establish about configured use.
#[derive(Debug)]
pub enum Usage {
    /// The peer cannot provide the profile list.
    Unavailable,
    /// A real profile list contains no reference to this route.
    Unused,
    /// A nonempty group; separate accounts are never collapsed by agent.
    Profiles {
        /// The first reported matching profile.
        first: User,
        /// Remaining matching profiles in node order.
        rest: Vec<User>,
    },
}
/// Resolve the row's use state without inferring general compatibility.
#[must_use]
pub fn usage(route: &ModelRoute, snapshot: &Snapshot) -> Usage {
    match &snapshot.profiles {
        Listed::OlderNode => Usage::Unavailable,
        Listed::Available(profiles) => {
            let mut users = super::users(route, profiles, &snapshot.agents).into_iter();
            match users.next() {
                Some(first) => Usage::Profiles {
                    first,
                    rest: users.collect(),
                },
                None => Usage::Unused,
            }
        }
    }
}
impl User {
    /// A human label is shown separately only when it adds to the identity.
    #[must_use]
    pub fn distinct_label(&self) -> Option<&str> {
        let label = self.label.as_str();
        (!label.trim().is_empty() && label != self.profile.as_str()).then_some(label)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn route() -> ModelRoute {
        serde_json::from_str(r#"{"name":"shared","provider":"fixture","api":"openai_responses","model":"fixture/model"}"#).unwrap()
    }
    #[test]
    fn missing_profiles_unused_and_two_accounts_are_three_distinct_states() {
        let route = route();
        let mut snapshot = Snapshot {
            node: vessel_proto::NodeId::random(),
            routes: Listed::Available(vec![route.clone()]),
            profiles: Listed::OlderNode,
            agents: vec![],
        };
        assert!(matches!(usage(&route, &snapshot), Usage::Unavailable));
        snapshot.profiles = Listed::Available(vec![]);
        assert!(matches!(usage(&route, &snapshot), Usage::Unused));
        snapshot.profiles=Listed::Available(serde_json::from_str(r#"[{"name":"work","label":"Work","harness":"codex","model":"shared"},{"name":"personal","harness":"codex","model":"shared"},{"name":"elsewhere","harness":"codex","model":"other"}]"#).unwrap());
        let Usage::Profiles { first, rest } = usage(&route, &snapshot) else {
            panic!("profile use missing")
        };
        assert_eq!(first.profile.as_str(), "work");
        assert_eq!(first.distinct_label(), Some("Work"));
        assert_eq!(
            rest.iter()
                .map(|user| user.profile.as_str())
                .collect::<Vec<_>>(),
            ["personal"]
        );
        assert_eq!(rest[0].distinct_label(), None);
        let mut blank = first;
        blank.label = "  ".into();
        assert_eq!(blank.distinct_label(), None);
    }
}
