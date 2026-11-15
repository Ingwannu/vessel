//! Bounded transient host hints, not persisted runtime addresses or authority.
use super::PluginThemeId;
use std::collections::VecDeque;
use vessel_proto::NodeId;

// Source uses a module Map. A native window bounds navigation-only history;
// evicting the least recent hint never removes a saved/current theme choice.
const MAX_PREFERRED_HOSTS: usize = 256;
/// Remember the host selected for each plugin theme during this window.
#[derive(Debug, Default)]
pub struct PreferredHosts(VecDeque<(PluginThemeId, NodeId)>);
impl PreferredHosts {
    /// Record only an eligible selection supplied by the plugin runtime owner.
    pub fn choose(&mut self, key: PluginThemeId, node: NodeId) {
        self.0.retain(|(known, _)| *known != key);
        if self.0.len() == MAX_PREFERRED_HOSTS {
            let _oldest = self.0.pop_front();
        }
        self.0.push_back((key, node));
    }
    /// An unavailable hint stays remembered; catalog resolution falls back.
    #[must_use]
    pub fn get(&self, key: &PluginThemeId) -> Option<&NodeId> {
        self.0
            .iter()
            .find_map(|(known, node)| (known == key).then_some(node))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_is_bounded_by_recency_without_becoming_selection_state() {
        let mut hosts = PreferredHosts::default();
        let node = NodeId::random();
        let first: PluginThemeId = "first/theme/dark".parse().unwrap();
        hosts.choose(first.clone(), node);
        for index in 0..MAX_PREFERRED_HOSTS {
            hosts.choose(format!("p{index}/theme/dark").parse().unwrap(), node);
        }
        assert_eq!(hosts.get(&first), None);
        assert_eq!(hosts.0.len(), MAX_PREFERRED_HOSTS);
    }
}
