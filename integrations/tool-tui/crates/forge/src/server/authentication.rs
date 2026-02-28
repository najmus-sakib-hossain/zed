use anyhow::{Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// User role for access control
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Developer,
    Viewer,
}

/// User account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub email: Option<String>,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

impl User {
    /// Create a new user with hashed password
    pub fn new(username: String, password: &str, role: Role) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            username,
            password_hash: hash_password(password),
            email: None,
            role,
            created_at: Utc::now(),
            last_login: None,
        }
    }

    /// Verify password
    pub fn verify_password(&self, password: &str) -> bool {
        self.password_hash == hash_password(password)
    }

    /// Check if user has permission
    pub fn has_permission(&self, required_role: &Role) -> bool {
        matches!(
            (&self.role, required_role),
            (Role::Admin, _)
                | (Role::Developer, Role::Developer | Role::Viewer)
                | (Role::Viewer, Role::Viewer)
        )
    }
}

/// Session token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub token: String,
    pub user_id: String,
    pub username: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session
    pub fn new(user: &User, duration_hours: i64) -> Self {
        let now = Utc::now();
        Self {
            token: Uuid::new_v4().to_string(),
            user_id: user.id.clone(),
            username: user.username.clone(),
            role: user.role.clone(),
            created_at: now,
            expires_at: now + Duration::hours(duration_hours),
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

/// Authentication manager
pub struct AuthManager {
    users: Arc<RwLock<HashMap<String, User>>>,
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Self {
        let mut users = HashMap::new();

        // Create default admin user
        let admin = User::new("admin".to_string(), "admin123", Role::Admin);
        users.insert(admin.username.clone(), admin);

        Self {
            users: Arc::new(RwLock::new(users)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a new user
    pub fn register(&self, username: String, password: &str, role: Role) -> Result<User> {
        let mut users = self.users.write();

        if users.contains_key(&username) {
            return Err(anyhow!("Username already exists"));
        }

        let user = User::new(username.clone(), password, role);
        users.insert(username, user.clone());

        Ok(user)
    }

    /// Authenticate user and create session
    pub fn login(&self, username: &str, password: &str) -> Result<Session> {
        let mut users = self.users.write();

        let user =
            users.get_mut(username).ok_or_else(|| anyhow!("Invalid username or password"))?;

        if !user.verify_password(password) {
            return Err(anyhow!("Invalid username or password"));
        }

        // Update last login
        user.last_login = Some(Utc::now());

        // Create session (24 hour duration)
        let session = Session::new(user, 24);

        let mut sessions = self.sessions.write();
        sessions.insert(session.token.clone(), session.clone());

        Ok(session)
    }

    /// Validate session token
    pub fn validate_token(&self, token: &str) -> Result<Session> {
        let mut sessions = self.sessions.write();

        let session = sessions.get(token).ok_or_else(|| anyhow!("Invalid or expired session"))?;

        if session.is_expired() {
            sessions.remove(token);
            return Err(anyhow!("Session expired"));
        }

        Ok(session.clone())
    }

    /// Logout and invalidate session
    pub fn logout(&self, token: &str) -> Result<()> {
        let mut sessions = self.sessions.write();
        sessions.remove(token);
        Ok(())
    }

    /// Get user by username
    pub fn get_user(&self, username: &str) -> Option<User> {
        let users = self.users.read();
        users.get(username).cloned()
    }

    /// List all users
    pub fn list_users(&self) -> Vec<User> {
        let users = self.users.read();
        users.values().cloned().collect()
    }

    /// Update user password
    pub fn update_password(
        &self,
        username: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        let mut users = self.users.write();

        let user = users.get_mut(username).ok_or_else(|| anyhow!("User not found"))?;

        if !user.verify_password(old_password) {
            return Err(anyhow!("Invalid current password"));
        }

        user.password_hash = hash_password(new_password);
        Ok(())
    }

    /// Delete user
    pub fn delete_user(&self, username: &str) -> Result<()> {
        let mut users = self.users.write();
        users.remove(username).ok_or_else(|| anyhow!("User not found"))?;
        Ok(())
    }

    /// Clean expired sessions
    pub fn clean_expired_sessions(&self) {
        let mut sessions = self.sessions.write();
        sessions.retain(|_, session| !session.is_expired());
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash password using SHA-256
fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
    pub role: Role,
    pub expires_at: DateTime<Utc>,
}

/// User creation request
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub role: Role,
    pub email: Option<String>,
}

/// Password change request
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}
