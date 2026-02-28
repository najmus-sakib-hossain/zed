//! Poll framework for interactive voting.
//!
//! Create polls, cast votes, and tally results across
//! any channel that supports interactive messages.

use anyhow::{Result, bail};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A poll with options and vote tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    /// Unique poll identifier.
    pub id: String,
    /// The question being asked.
    pub question: String,
    /// Available options (ordered).
    pub options: Vec<String>,
    /// Vote counts per option index.
    pub votes: HashMap<usize, u64>,
    /// Set of user IDs that have voted (to prevent duplicates).
    #[serde(default)]
    pub voters: HashSet<String>,
    /// Whether multiple choices are allowed.
    pub multi_select: bool,
    /// Whether votes are anonymous.
    pub anonymous: bool,
    /// When the poll was created.
    pub created_at: DateTime<Utc>,
    /// Optional expiry time.
    pub expires_at: Option<DateTime<Utc>>,
    /// Whether the poll is closed.
    pub closed: bool,
}

/// Manages polls across channels.
#[derive(Clone)]
pub struct PollManager {
    polls: Arc<DashMap<String, Poll>>,
}

impl PollManager {
    /// Create a new, empty poll manager.
    pub fn new() -> Self {
        Self {
            polls: Arc::new(DashMap::new()),
        }
    }

    /// Create a new poll.
    ///
    /// Returns the generated poll ID.
    pub fn create_poll(&self, question: impl Into<String>, options: Vec<String>) -> Result<String> {
        if options.is_empty() {
            bail!("Poll must have at least one option");
        }
        if options.len() > 20 {
            bail!("Poll cannot have more than 20 options");
        }

        let id = format!("poll-{}", uuid::Uuid::new_v4().as_simple());
        let mut votes = HashMap::new();
        for i in 0..options.len() {
            votes.insert(i, 0);
        }

        let poll = Poll {
            id: id.clone(),
            question: question.into(),
            options,
            votes,
            voters: HashSet::new(),
            multi_select: false,
            anonymous: false,
            created_at: Utc::now(),
            expires_at: None,
            closed: false,
        };

        self.polls.insert(id.clone(), poll);
        Ok(id)
    }

    /// Create a poll with advanced options.
    pub fn create_poll_advanced(
        &self,
        question: impl Into<String>,
        options: Vec<String>,
        multi_select: bool,
        anonymous: bool,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<String> {
        if options.is_empty() {
            bail!("Poll must have at least one option");
        }

        let id = format!("poll-{}", uuid::Uuid::new_v4().as_simple());
        let mut votes = HashMap::new();
        for i in 0..options.len() {
            votes.insert(i, 0);
        }

        let poll = Poll {
            id: id.clone(),
            question: question.into(),
            options,
            votes,
            voters: HashSet::new(),
            multi_select,
            anonymous,
            created_at: Utc::now(),
            expires_at,
            closed: false,
        };

        self.polls.insert(id.clone(), poll);
        Ok(id)
    }

    /// Cast a vote on a poll.
    pub fn vote(&self, poll_id: &str, option_index: usize, user_id: &str) -> Result<()> {
        let mut entry = self
            .polls
            .get_mut(poll_id)
            .ok_or_else(|| anyhow::anyhow!("Poll not found: {}", poll_id))?;

        let poll = entry.value_mut();

        if poll.closed {
            bail!("Poll '{}' is closed", poll_id);
        }

        if poll.expires_at.is_some_and(|exp| Utc::now() > exp) {
            poll.closed = true;
            bail!("Poll '{}' has expired", poll_id);
        }

        if option_index >= poll.options.len() {
            bail!(
                "Invalid option index {} (poll has {} options)",
                option_index,
                poll.options.len()
            );
        }

        if !poll.multi_select && poll.voters.contains(user_id) {
            bail!("User '{}' has already voted", user_id);
        }

        *poll.votes.entry(option_index).or_insert(0) += 1;
        poll.voters.insert(user_id.to_string());

        Ok(())
    }

    /// Get vote results for a poll.
    ///
    /// Returns `option_text → vote_count`.
    pub fn get_results(&self, poll_id: &str) -> Option<HashMap<String, u64>> {
        self.polls.get(poll_id).map(|entry| {
            let poll = entry.value();
            let mut results = HashMap::new();
            for (idx, count) in &poll.votes {
                if let Some(option) = poll.options.get(*idx) {
                    results.insert(option.clone(), *count);
                }
            }
            results
        })
    }

    /// Get a poll snapshot.
    pub fn get_poll(&self, poll_id: &str) -> Option<Poll> {
        self.polls.get(poll_id).map(|r| r.value().clone())
    }

    /// Close a poll so no more votes can be cast.
    pub fn close_poll(&self, poll_id: &str) -> Result<()> {
        let mut entry = self
            .polls
            .get_mut(poll_id)
            .ok_or_else(|| anyhow::anyhow!("Poll not found: {}", poll_id))?;
        entry.value_mut().closed = true;
        Ok(())
    }

    /// Remove a poll.
    pub fn remove_poll(&self, poll_id: &str) -> Option<Poll> {
        self.polls.remove(poll_id).map(|(_, v)| v)
    }

    /// List all open (non-closed) polls.
    pub fn list_open(&self) -> Vec<Poll> {
        self.polls
            .iter()
            .filter(|r| !r.value().closed)
            .map(|r| r.value().clone())
            .collect()
    }

    /// Total tracked polls.
    pub fn count(&self) -> usize {
        self.polls.len()
    }
}

impl Default for PollManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_poll() {
        let mgr = PollManager::new();
        let id = mgr
            .create_poll("Favorite color?", vec!["Red".into(), "Blue".into(), "Green".into()])
            .expect("should create");

        let poll = mgr.get_poll(&id).expect("should exist");
        assert_eq!(poll.question, "Favorite color?");
        assert_eq!(poll.options.len(), 3);
        assert!(!poll.closed);
    }

    #[test]
    fn test_create_poll_empty_options() {
        let mgr = PollManager::new();
        let result = mgr.create_poll("Q?", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_vote() {
        let mgr = PollManager::new();
        let id = mgr.create_poll("Q?", vec!["A".into(), "B".into()]).expect("create");

        mgr.vote(&id, 0, "user1").expect("vote");
        mgr.vote(&id, 1, "user2").expect("vote");
        mgr.vote(&id, 0, "user3").expect("vote");

        let results = mgr.get_results(&id).expect("results");
        assert_eq!(*results.get("A").unwrap_or(&0), 2);
        assert_eq!(*results.get("B").unwrap_or(&0), 1);
    }

    #[test]
    fn test_vote_duplicate_prevented() {
        let mgr = PollManager::new();
        let id = mgr.create_poll("Q?", vec!["A".into()]).expect("create");

        mgr.vote(&id, 0, "user1").expect("first vote");
        let result = mgr.vote(&id, 0, "user1");
        assert!(result.is_err());
    }

    #[test]
    fn test_vote_invalid_option() {
        let mgr = PollManager::new();
        let id = mgr.create_poll("Q?", vec!["A".into()]).expect("create");

        let result = mgr.vote(&id, 5, "user1");
        assert!(result.is_err());
    }

    #[test]
    fn test_close_poll() {
        let mgr = PollManager::new();
        let id = mgr.create_poll("Q?", vec!["A".into()]).expect("create");

        mgr.close_poll(&id).expect("close");
        let result = mgr.vote(&id, 0, "user1");
        assert!(result.is_err());
    }

    #[test]
    fn test_list_open() {
        let mgr = PollManager::new();
        let id1 = mgr.create_poll("Q1?", vec!["A".into()]).expect("create");
        mgr.create_poll("Q2?", vec!["B".into()]).expect("create");

        mgr.close_poll(&id1).expect("close");

        let open = mgr.list_open();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].question, "Q2?");
    }

    #[test]
    fn test_remove_poll() {
        let mgr = PollManager::new();
        let id = mgr.create_poll("Q?", vec!["A".into()]).expect("create");
        assert_eq!(mgr.count(), 1);

        mgr.remove_poll(&id);
        assert_eq!(mgr.count(), 0);
    }
}
