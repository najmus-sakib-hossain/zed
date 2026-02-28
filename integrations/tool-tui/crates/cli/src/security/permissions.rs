//! Permission System
//!
//! Fine-grained permission management with capability-based security.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime};

use super::{Capability, SecurityError, TrustLevel};

/// Permission grant
#[derive(Debug, Clone)]
pub struct Permission {
    /// Unique ID
    pub id: String,
    /// Granted capability
    pub capability: Capability,
    /// Resource pattern (glob)
    pub resource: String,
    /// Expiration time
    pub expires_at: Option<SystemTime>,
    /// Maximum uses (None = unlimited)
    pub max_uses: Option<u32>,
    /// Current use count
    pub use_count: u32,
    /// Granted by (user/system)
    pub granted_by: String,
    /// Grant reason
    pub reason: String,
}

impl Permission {
    /// Create new permission
    pub fn new(capability: Capability, resource: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            capability,
            resource: resource.to_string(),
            expires_at: None,
            max_uses: None,
            use_count: 0,
            granted_by: "system".into(),
            reason: String::new(),
        }
    }

    /// Set expiration
    pub fn expires_in(mut self, duration: Duration) -> Self {
        self.expires_at = Some(SystemTime::now() + duration);
        self
    }

    /// Set max uses
    pub fn max_uses(mut self, count: u32) -> Self {
        self.max_uses = Some(count);
        self
    }

    /// Set granted by
    pub fn granted_by(mut self, by: &str) -> Self {
        self.granted_by = by.to_string();
        self
    }

    /// Set reason
    pub fn with_reason(mut self, reason: &str) -> Self {
        self.reason = reason.to_string();
        self
    }

    /// Check if permission is valid
    pub fn is_valid(&self) -> bool {
        // Check expiration
        if let Some(expires) = self.expires_at {
            if SystemTime::now() > expires {
                return false;
            }
        }

        // Check use count
        if let Some(max) = self.max_uses {
            if self.use_count >= max {
                return false;
            }
        }

        true
    }

    /// Use the permission (increment counter)
    pub fn use_permission(&mut self) -> bool {
        if !self.is_valid() {
            return false;
        }
        self.use_count += 1;
        true
    }

    /// Check if resource matches pattern
    pub fn matches_resource(&self, resource: &str) -> bool {
        if self.resource == "*" {
            return true;
        }

        // Simple glob matching
        if self.resource.ends_with("/*") {
            let prefix = &self.resource[..self.resource.len() - 2];
            return resource.starts_with(prefix);
        }

        if self.resource.ends_with("/**") {
            let prefix = &self.resource[..self.resource.len() - 3];
            return resource.starts_with(prefix);
        }

        self.resource == resource
    }
}

/// Permission manager
pub struct PermissionManager {
    /// Permissions by context
    permissions: HashMap<String, Vec<Permission>>,
    /// Trust levels by context
    trust_levels: HashMap<String, TrustLevel>,
    /// Pending permission requests
    pending_requests: Vec<PermissionRequest>,
    /// Permission policies
    policies: Vec<PermissionPolicy>,
}

/// Permission request
#[derive(Debug, Clone)]
pub struct PermissionRequest {
    /// Request ID
    pub id: String,
    /// Requesting context
    pub context: String,
    /// Requested capability
    pub capability: Capability,
    /// Requested resource
    pub resource: String,
    /// Reason for request
    pub reason: String,
    /// Request time
    pub requested_at: SystemTime,
    /// Status
    pub status: RequestStatus,
}

/// Request status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
}

/// Permission policy
#[derive(Debug, Clone)]
pub struct PermissionPolicy {
    /// Policy name
    pub name: String,
    /// Minimum trust level required
    pub min_trust: TrustLevel,
    /// Auto-grant these capabilities
    pub auto_grant: HashSet<Capability>,
    /// Always deny these capabilities
    pub always_deny: HashSet<Capability>,
    /// Require approval for these
    pub require_approval: HashSet<Capability>,
}

impl Default for PermissionPolicy {
    fn default() -> Self {
        Self {
            name: "default".into(),
            min_trust: TrustLevel::Basic,
            auto_grant: HashSet::from([Capability::FileRead]),
            always_deny: HashSet::from([Capability::SystemCall]),
            require_approval: HashSet::from([Capability::NetworkListen, Capability::EnvAccess]),
        }
    }
}

impl PermissionManager {
    /// Create new permission manager
    pub fn new() -> Self {
        Self {
            permissions: HashMap::new(),
            trust_levels: HashMap::new(),
            pending_requests: Vec::new(),
            policies: vec![PermissionPolicy::default()],
        }
    }

    /// Set trust level for context
    pub fn set_trust_level(&mut self, context: &str, level: TrustLevel) {
        self.trust_levels.insert(context.to_string(), level);
    }

    /// Get trust level for context
    pub fn get_trust_level(&self, context: &str) -> TrustLevel {
        self.trust_levels.get(context).copied().unwrap_or(TrustLevel::Untrusted)
    }

    /// Grant permission
    pub fn grant(&mut self, context: &str, permission: Permission) -> String {
        let id = permission.id.clone();
        self.permissions.entry(context.to_string()).or_default().push(permission);
        id
    }

    /// Revoke permission by ID
    pub fn revoke(&mut self, context: &str, permission_id: &str) -> bool {
        if let Some(perms) = self.permissions.get_mut(context) {
            let len_before = perms.len();
            perms.retain(|p| p.id != permission_id);
            return perms.len() < len_before;
        }
        false
    }

    /// Revoke all permissions for context
    pub fn revoke_all(&mut self, context: &str) {
        self.permissions.remove(context);
    }

    /// Check permission
    pub fn check(
        &self,
        context: &str,
        capability: Capability,
        resource: &str,
    ) -> Result<(), SecurityError> {
        let trust_level = self.get_trust_level(context);

        // Check policies first
        for policy in &self.policies {
            if policy.always_deny.contains(&capability) {
                return Err(SecurityError::PermissionDenied(format!(
                    "Capability {:?} is always denied by policy '{}'",
                    capability, policy.name
                )));
            }

            if trust_level >= policy.min_trust && policy.auto_grant.contains(&capability) {
                return Ok(());
            }
        }

        // Check trust level capabilities
        if trust_level.capabilities().contains(&capability) {
            return Ok(());
        }

        // Check explicit permissions
        if let Some(perms) = self.permissions.get(context) {
            for perm in perms {
                if perm.capability == capability
                    && perm.matches_resource(resource)
                    && perm.is_valid()
                {
                    return Ok(());
                }
            }
        }

        Err(SecurityError::PermissionDenied(format!(
            "No permission for {:?} on '{}'",
            capability, resource
        )))
    }

    /// Use permission (for limited-use permissions)
    pub fn use_permission(
        &mut self,
        context: &str,
        capability: Capability,
        resource: &str,
    ) -> Result<(), SecurityError> {
        // First check if allowed
        self.check(context, capability, resource)?;

        // Then try to use explicit permission
        if let Some(perms) = self.permissions.get_mut(context) {
            for perm in perms {
                if perm.capability == capability
                    && perm.matches_resource(resource)
                    && perm.is_valid()
                {
                    perm.use_permission();
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    /// Request permission
    pub fn request(
        &mut self,
        context: &str,
        capability: Capability,
        resource: &str,
        reason: &str,
    ) -> String {
        let request = PermissionRequest {
            id: uuid::Uuid::new_v4().to_string(),
            context: context.to_string(),
            capability,
            resource: resource.to_string(),
            reason: reason.to_string(),
            requested_at: SystemTime::now(),
            status: RequestStatus::Pending,
        };

        let id = request.id.clone();
        self.pending_requests.push(request);
        id
    }

    /// Approve request
    pub fn approve_request(&mut self, request_id: &str, duration: Option<Duration>) -> bool {
        // Extract data from request first to avoid borrow conflict
        let grant_data = self
            .pending_requests
            .iter_mut()
            .find(|r| r.id == request_id)
            .filter(|r| r.status == RequestStatus::Pending)
            .map(|req| {
                req.status = RequestStatus::Approved;
                (req.capability, req.resource.clone(), req.reason.clone(), req.context.clone())
            });

        if let Some((capability, resource, reason, context)) = grant_data {
            let mut perm = Permission::new(capability, &resource).with_reason(&reason);

            if let Some(dur) = duration {
                perm = perm.expires_in(dur);
            }

            self.grant(&context, perm);
            return true;
        }
        false
    }

    /// Deny request
    pub fn deny_request(&mut self, request_id: &str) -> bool {
        if let Some(req) = self.pending_requests.iter_mut().find(|r| r.id == request_id) {
            if req.status == RequestStatus::Pending {
                req.status = RequestStatus::Denied;
                return true;
            }
        }
        false
    }

    /// Get pending requests
    pub fn pending_requests(&self) -> Vec<&PermissionRequest> {
        self.pending_requests
            .iter()
            .filter(|r| r.status == RequestStatus::Pending)
            .collect()
    }

    /// Cleanup expired permissions
    pub fn cleanup(&mut self) {
        for perms in self.permissions.values_mut() {
            perms.retain(|p| p.is_valid());
        }

        // Expire old requests
        let cutoff = SystemTime::now() - Duration::from_secs(24 * 60 * 60);
        for req in &mut self.pending_requests {
            if req.status == RequestStatus::Pending && req.requested_at < cutoff {
                req.status = RequestStatus::Expired;
            }
        }
    }

    /// Add policy
    pub fn add_policy(&mut self, policy: PermissionPolicy) {
        self.policies.push(policy);
    }

    /// List all permissions for context
    pub fn list_permissions(&self, context: &str) -> Vec<&Permission> {
        self.permissions
            .get(context)
            .map(|p| p.iter().filter(|p| p.is_valid()).collect())
            .unwrap_or_default()
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_validity() {
        let mut perm = Permission::new(Capability::FileRead, "/test/*").max_uses(2);

        assert!(perm.is_valid());
        assert!(perm.use_permission());
        assert!(perm.use_permission());
        assert!(!perm.use_permission()); // Exceeded max uses
        assert!(!perm.is_valid());
    }

    #[test]
    fn test_resource_matching() {
        let perm = Permission::new(Capability::FileRead, "/project/*");

        assert!(perm.matches_resource("/project/file.rs"));
        assert!(!perm.matches_resource("/other/file.rs"));
    }

    #[test]
    fn test_permission_manager() {
        let mut manager = PermissionManager::new();
        manager.set_trust_level("test", TrustLevel::Standard);

        // Should auto-grant FileRead for Standard trust
        assert!(manager.check("test", Capability::FileRead, "/any").is_ok());

        // Should allow FileWrite for Standard trust
        assert!(manager.check("test", Capability::FileWrite, "/any").is_ok());

        // Should deny NetworkListen without explicit grant
        assert!(manager.check("test", Capability::NetworkListen, "localhost:8080").is_err());
    }

    #[test]
    fn test_explicit_permission() {
        let mut manager = PermissionManager::new();
        manager.set_trust_level("test", TrustLevel::Basic);

        // Basic trust doesn't have FileWrite
        assert!(manager.check("test", Capability::FileWrite, "/project/file.rs").is_err());

        // Grant explicit permission
        manager.grant("test", Permission::new(Capability::FileWrite, "/project/*"));

        // Now should be allowed
        assert!(manager.check("test", Capability::FileWrite, "/project/file.rs").is_ok());
    }
}
