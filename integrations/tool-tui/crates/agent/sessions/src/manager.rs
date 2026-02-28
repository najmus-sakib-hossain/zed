//! Session manager for handling multiple concurrent sessions.

use anyhow::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

use crate::context::ContextWindow;
use crate::storage::SessionStorage;

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    Active,
    Idle,
    Suspended,
    Terminated,
}

/// Configuration for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum context window tokens
    pub max_tokens: u32,
    /// Reserve tokens for response
    pub reserve_tokens: u32,
    /// Session timeout in seconds
    pub timeout_secs: u64,
    /// Auto-compact threshold (number of messages)
    pub compact_threshold: usize,
    /// System prompt
    pub system_prompt: Option<String>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_tokens: 128_000,
            reserve_tokens: 4_096,
            timeout_secs: 3600,
            compact_threshold: 50,
            system_prompt: None,
        }
    }
}

/// A user session with context and metadata
#[derive(Debug)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID that owns this session
    pub user_id: String,
    /// Channel (telegram, discord, etc.)
    pub channel: String,
    /// Chat ID within the channel
    pub chat_id: String,
    /// Session state
    pub state: SessionState,
    /// Context window
    pub context: ContextWindow,
    /// Configuration
    pub config: SessionConfig,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Session metadata
    pub metadata: std::collections::HashMap<String, String>,
    /// Assigned agent ID (for multi-agent routing)
    pub agent_id: Option<String>,
}

/// Agent routing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Round-robin across available agents
    RoundRobin,
    /// Route by channel type (each channel gets a dedicated agent)
    ByChannel,
    /// Route by user affinity (same user always gets same agent)
    UserAffinity,
    /// Route by load (least-busy agent)
    LeastLoaded,
    /// Manual routing (agent explicitly assigned)
    Manual,
}

impl Default for RoutingStrategy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Multi-agent router for distributing sessions across agents
pub struct AgentRouter {
    /// Available agent IDs
    agents: Vec<String>,
    /// Routing strategy
    strategy: RoutingStrategy,
    /// Round-robin counter
    rr_counter: std::sync::atomic::AtomicUsize,
    /// User -> agent affinity map
    user_affinity: DashMap<String, String>,
    /// Agent -> session count for load balancing
    agent_load: DashMap<String, usize>,
}

impl AgentRouter {
    /// Create a new agent router
    pub fn new(agents: Vec<String>, strategy: RoutingStrategy) -> Self {
        let agent_load = DashMap::new();
        for agent in &agents {
            agent_load.insert(agent.clone(), 0);
        }
        Self {
            agents,
            strategy,
            rr_counter: std::sync::atomic::AtomicUsize::new(0),
            user_affinity: DashMap::new(),
            agent_load,
        }
    }

    /// Route a session to an agent
    pub fn route(&self, user_id: &str, channel: &str) -> Option<String> {
        if self.agents.is_empty() {
            return None;
        }

        match self.strategy {
            RoutingStrategy::RoundRobin => {
                let idx = self.rr_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    % self.agents.len();
                let agent = self.agents[idx].clone();
                self.increment_load(&agent);
                Some(agent)
            }
            RoutingStrategy::ByChannel => {
                // Hash channel name to pick agent deterministically
                let hash = channel.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize));
                let idx = hash % self.agents.len();
                Some(self.agents[idx].clone())
            }
            RoutingStrategy::UserAffinity => {
                if let Some(agent) = self.user_affinity.get(user_id) {
                    return Some(agent.clone());
                }
                // Assign via round-robin for new users
                let idx = self.rr_counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                    % self.agents.len();
                let agent = self.agents[idx].clone();
                self.user_affinity.insert(user_id.to_string(), agent.clone());
                self.increment_load(&agent);
                Some(agent)
            }
            RoutingStrategy::LeastLoaded => {
                let mut min_load = usize::MAX;
                let mut best_agent = self.agents[0].clone();
                for entry in self.agent_load.iter() {
                    if *entry.value() < min_load {
                        min_load = *entry.value();
                        best_agent = entry.key().clone();
                    }
                }
                self.increment_load(&best_agent);
                Some(best_agent)
            }
            RoutingStrategy::Manual => None,
        }
    }

    /// Release an agent assignment (session ended)
    pub fn release(&self, agent_id: &str) {
        if let Some(mut load) = self.agent_load.get_mut(agent_id) {
            *load = load.saturating_sub(1);
        }
    }

    /// Add a new agent to the pool
    pub fn add_agent(&mut self, agent_id: String) {
        self.agent_load.insert(agent_id.clone(), 0);
        self.agents.push(agent_id);
    }

    /// Remove an agent from the pool
    pub fn remove_agent(&mut self, agent_id: &str) {
        self.agents.retain(|a| a != agent_id);
        self.agent_load.remove(agent_id);
    }

    /// Get agent session counts
    pub fn load_summary(&self) -> Vec<(String, usize)> {
        self.agent_load.iter().map(|e| (e.key().clone(), *e.value())).collect()
    }

    fn increment_load(&self, agent_id: &str) {
        if let Some(mut load) = self.agent_load.get_mut(agent_id) {
            *load += 1;
        }
    }
}

impl Session {
    pub fn new(user_id: String, channel: String, chat_id: String, config: SessionConfig) -> Self {
        let mut context = ContextWindow::new(config.max_tokens, config.reserve_tokens);
        if let Some(ref prompt) = config.system_prompt {
            let tokens = crate::context::estimate_tokens(prompt);
            context.set_system_prompt(prompt.clone(), tokens);
        }

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            channel,
            chat_id,
            state: SessionState::Active,
            context,
            config,
            created_at: Utc::now(),
            last_activity: Utc::now(),
            metadata: std::collections::HashMap::new(),
            agent_id: None,
        }
    }

    /// Touch the session (update last activity)
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
        if self.state == SessionState::Idle {
            self.state = SessionState::Active;
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.last_activity).num_seconds() as u64;
        elapsed > self.config.timeout_secs
    }
}

/// Manages multiple concurrent sessions
pub struct SessionManager {
    /// Active sessions indexed by session ID
    sessions: DashMap<String, Session>,
    /// Mapping from (channel, chat_id) -> session_id
    channel_index: DashMap<(String, String), String>,
    /// Storage backend
    storage: Option<Arc<dyn SessionStorage>>,
    /// Default session configuration
    default_config: SessionConfig,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(default_config: SessionConfig) -> Self {
        Self {
            sessions: DashMap::new(),
            channel_index: DashMap::new(),
            storage: None,
            default_config,
        }
    }

    /// Set the storage backend
    pub fn with_storage(mut self, storage: Arc<dyn SessionStorage>) -> Self {
        self.storage = Some(storage);
        self
    }

    /// Get or create a session for a channel + chat
    pub fn get_or_create(&self, user_id: &str, channel: &str, chat_id: &str) -> String {
        let key = (channel.to_string(), chat_id.to_string());

        // Check existing
        if let Some(session_id) = self.channel_index.get(&key) {
            if let Some(mut session) = self.sessions.get_mut(session_id.value()) {
                if !session.is_expired() {
                    session.touch();
                    return session.id.clone();
                }
                // Expired - remove it
                let id = session.id.clone();
                drop(session);
                self.sessions.remove(&id);
            }
            self.channel_index.remove(&key);
        }

        // Create new session
        let session = Session::new(
            user_id.to_string(),
            channel.to_string(),
            chat_id.to_string(),
            self.default_config.clone(),
        );
        let id = session.id.clone();
        self.channel_index.insert(key, id.clone());
        self.sessions.insert(id.clone(), session);
        info!("Created session {} for {}:{}", id, channel, chat_id);
        id
    }

    /// Get a session by ID
    pub fn get(&self, session_id: &str) -> Option<dashmap::mapref::one::Ref<String, Session>> {
        self.sessions.get(session_id)
    }

    /// Get a mutable session by ID
    pub fn get_mut(
        &self,
        session_id: &str,
    ) -> Option<dashmap::mapref::one::RefMut<String, Session>> {
        self.sessions.get_mut(session_id)
    }

    /// Terminate a session
    pub fn terminate(&self, session_id: &str) {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.state = SessionState::Terminated;
            let key = (session.channel.clone(), session.chat_id.clone());
            drop(session);
            self.channel_index.remove(&key);
            self.sessions.remove(session_id);
            info!("Terminated session {}", session_id);
        }
    }

    /// List all active sessions
    pub fn list_active(&self) -> Vec<String> {
        self.sessions
            .iter()
            .filter(|s| s.state == SessionState::Active || s.state == SessionState::Idle)
            .map(|s| s.id.clone())
            .collect()
    }

    /// Count active sessions
    pub fn count(&self) -> usize {
        self.sessions.len()
    }

    /// Clean up expired sessions
    pub fn cleanup_expired(&self) -> usize {
        let expired: Vec<String> =
            self.sessions.iter().filter(|s| s.is_expired()).map(|s| s.id.clone()).collect();
        let count = expired.len();
        for id in &expired {
            self.terminate(id);
        }
        if count > 0 {
            warn!("Cleaned up {} expired sessions", count);
        }
        count
    }

    /// Persist all sessions to storage
    pub async fn persist_all(&self) -> Result<()> {
        if let Some(ref storage) = self.storage {
            for session in self.sessions.iter() {
                storage.save_session(&session).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let config = SessionConfig::default();
        let session = Session::new("user1".into(), "telegram".into(), "chat1".into(), config);
        assert_eq!(session.state, SessionState::Active);
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_manager() {
        let manager = SessionManager::new(SessionConfig::default());

        let id1 = manager.get_or_create("user1", "telegram", "chat1");
        let id2 = manager.get_or_create("user1", "telegram", "chat1");
        assert_eq!(id1, id2); // Same session

        let id3 = manager.get_or_create("user1", "discord", "chat2");
        assert_ne!(id1, id3); // Different session

        assert_eq!(manager.count(), 2);
        manager.terminate(&id1);
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_session_touch() {
        let config = SessionConfig::default();
        let mut session = Session::new("user1".into(), "telegram".into(), "chat1".into(), config);
        session.state = SessionState::Idle;
        session.touch();
        assert_eq!(session.state, SessionState::Active);
    }
}
