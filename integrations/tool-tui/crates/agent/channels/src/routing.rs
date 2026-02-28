//! Group routing infrastructure for channel dispatch.
//!
//! Routes group/channel IDs to specific agent targets and fallback channels.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Routing target for a group/chat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupRoute {
    pub group_id: String,
    pub channel: String,
    pub agent_id: Option<String>,
    pub fallback_channel: Option<String>,
}

/// In-memory group router (can be persisted by caller if needed).
#[derive(Default)]
pub struct GroupRouter {
    routes: DashMap<String, GroupRoute>,
}

impl GroupRouter {
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
        }
    }

    pub fn upsert_route(&self, route: GroupRoute) {
        self.routes.insert(route.group_id.clone(), route);
    }

    pub fn remove_route(&self, group_id: &str) -> Option<GroupRoute> {
        self.routes.remove(group_id).map(|(_, route)| route)
    }

    pub fn get_route(&self, group_id: &str) -> Option<GroupRoute> {
        self.routes.get(group_id).map(|r| r.value().clone())
    }

    pub fn resolve_channel(&self, group_id: &str) -> Option<String> {
        self.routes.get(group_id).map(|r| r.channel.clone())
    }

    pub fn resolve_agent(&self, group_id: &str) -> Option<String> {
        self.routes.get(group_id).and_then(|r| r.agent_id.clone())
    }

    pub fn list_routes(&self) -> Vec<GroupRoute> {
        self.routes.iter().map(|r| r.value().clone()).collect()
    }

    pub fn count(&self) -> usize {
        self.routes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_and_get_route() {
        let router = GroupRouter::new();
        router.upsert_route(GroupRoute {
            group_id: "g1".into(),
            channel: "telegram".into(),
            agent_id: Some("agent-a".into()),
            fallback_channel: Some("slack".into()),
        });

        let route = router.get_route("g1").expect("route exists");
        assert_eq!(route.channel, "telegram");
        assert_eq!(route.agent_id.as_deref(), Some("agent-a"));
    }

    #[test]
    fn remove_route() {
        let router = GroupRouter::new();
        router.upsert_route(GroupRoute {
            group_id: "g2".into(),
            channel: "discord".into(),
            agent_id: None,
            fallback_channel: None,
        });

        assert_eq!(router.count(), 1);
        let removed = router.remove_route("g2");
        assert!(removed.is_some());
        assert_eq!(router.count(), 0);
    }
}
