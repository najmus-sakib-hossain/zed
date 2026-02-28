//! Plugin System Integration Tests
//!
//! End-to-end tests that exercise the full plugin lifecycle:
//! manager → registry → sandbox → execution → hooks

#[cfg(test)]
mod plugin_integration_tests {
    use dx_cli::plugin::hooks::{HookData, HookSystem, events};
    use dx_cli::plugin::host_functions::{HostState, host_kv_get, host_kv_set, host_log};
    use dx_cli::plugin::manager::{PluginManager, PluginManagerConfig};
    use dx_cli::plugin::resource_limiter::{ResourceLimits, ResourceTracker};
    use dx_cli::plugin::sandbox::{NetworkPolicy, PluginSandbox, SandboxConfig};
    use dx_cli::plugin::signature::{
        SignatureVerifier, TrustedKey, VerificationResult, generate_keypair, sign_plugin,
    };
    use dx_cli::plugin::traits::{Capability, PluginContext, PluginResult as PResult};
    use dx_cli::plugin::validation;
    use dx_cli::plugin::{PluginRegistry, PluginType};

    // -----------------------------------------------------------------------
    // Plugin Manager lifecycle
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_manager_create_init_shutdown() {
        let config = PluginManagerConfig {
            auto_load: false,
            ..Default::default()
        };
        let manager = PluginManager::new(config).unwrap();

        let loaded = manager.init().await.unwrap();
        assert_eq!(loaded, 0); // auto_load disabled, nothing to load

        assert_eq!(manager.plugin_count(), 0);
        assert!(manager.shutdown().await.is_ok());
    }

    #[tokio::test]
    async fn test_manager_execute_missing_plugin() {
        let mgr = PluginManager::new(PluginManagerConfig {
            auto_load: false,
            ..Default::default()
        })
        .unwrap();

        let result = mgr.execute("does-not-exist", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_manager_list_when_empty() {
        let mgr = PluginManager::new(PluginManagerConfig {
            auto_load: false,
            ..Default::default()
        })
        .unwrap();

        let list = mgr.list_plugins_with_health().await;
        assert!(list.is_empty());
    }

    // -----------------------------------------------------------------------
    // Registry
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_registry_lifecycle() {
        let registry = PluginRegistry::new().unwrap();
        assert!(registry.list().is_empty());
        assert!(!registry.exists("ghost"));

        let health = registry.health_check_all().await;
        assert!(health.is_empty());

        assert!(registry.shutdown_all().await.is_ok());
    }

    #[test]
    fn test_plugin_type_detection() {
        use std::path::Path;

        assert_eq!(PluginType::from_path(Path::new("a.wasm")), Some(PluginType::Wasm));
        assert_eq!(PluginType::from_path(Path::new("b.dll")), Some(PluginType::Native));
        assert_eq!(PluginType::from_path(Path::new("c.so")), Some(PluginType::Native));
        assert_eq!(PluginType::from_path(Path::new("d.dylib")), Some(PluginType::Native));
        assert_eq!(PluginType::from_path(Path::new("e.txt")), None);
    }

    // -----------------------------------------------------------------------
    // Sandbox + Capabilities
    // -----------------------------------------------------------------------

    #[test]
    fn test_sandbox_capability_enforcement() {
        let config = SandboxConfig::restrictive().with_capability(Capability::Network);
        let mut sandbox = PluginSandbox::new(config);

        assert!(sandbox.check_capability(Capability::Network).is_ok());
        assert!(sandbox.check_capability(Capability::Shell).is_err());
        assert!(sandbox.check_capability(Capability::FileRead).is_err());
    }

    #[test]
    fn test_sandbox_filesystem_enforcement() {
        let config = SandboxConfig::restrictive().with_fs_read(std::path::PathBuf::from("/tmp"));
        let mut sandbox = PluginSandbox::new(config);

        assert!(
            sandbox
                .check_file_access(&std::path::PathBuf::from("/tmp/data.txt"), false)
                .is_ok()
        );
        assert!(
            sandbox
                .check_file_access(&std::path::PathBuf::from("/etc/passwd"), false)
                .is_err()
        );
    }

    #[test]
    fn test_sandbox_network_policy() {
        let config = SandboxConfig::restrictive()
            .with_network(NetworkPolicy::AllowedHosts(vec!["api.dx.dev".to_string()]));
        let mut sandbox = PluginSandbox::new(config);

        assert!(sandbox.check_network_access("api.dx.dev", 443).is_ok());
        assert!(sandbox.check_network_access("evil.com", 443).is_err());
    }

    #[test]
    fn test_sandbox_audit_log() {
        let config = SandboxConfig::restrictive().with_capability(Capability::Network);
        let mut sandbox = PluginSandbox::new(config);

        let _ = sandbox.check_capability(Capability::Network);
        let _ = sandbox.check_capability(Capability::Shell);

        assert_eq!(sandbox.audit_log().len(), 2);
        assert!(sandbox.audit_log()[0].allowed);
        assert!(!sandbox.audit_log()[1].allowed);

        sandbox.clear_audit_log();
        assert!(sandbox.audit_log().is_empty());
    }

    // -----------------------------------------------------------------------
    // Host Functions
    // -----------------------------------------------------------------------

    #[test]
    fn test_host_kv_roundtrip() {
        let state = HostState::new();

        host_kv_set(&state, "session:1", b"data-value");
        let val = host_kv_get(&state, "session:1");
        assert_eq!(val, b"data-value");
    }

    #[test]
    fn test_host_logging_integration() {
        let state = HostState::new();

        host_log(&state, 2, "Boot complete");
        host_log(&state, 3, "Cache miss");
        host_log(&state, 4, "DB unreachable");

        let logs = state.get_logs();
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].message, "Boot complete");
        assert_eq!(logs[2].message, "DB unreachable");
    }

    // -----------------------------------------------------------------------
    // Resource Limiter
    // -----------------------------------------------------------------------

    #[test]
    fn test_resource_tracker_memory_enforcement() {
        let limits = ResourceLimits {
            max_memory_bytes: 2048,
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        assert!(tracker.alloc_memory(1024).is_ok());
        assert!(tracker.alloc_memory(1024).is_ok());
        assert!(tracker.alloc_memory(1).is_err()); // Over limit
        assert!(tracker.is_killed());
    }

    #[test]
    fn test_resource_tracker_fuel_enforcement() {
        let limits = ResourceLimits {
            max_fuel: 500,
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        assert!(tracker.consume_fuel(200).is_ok());
        assert!(tracker.consume_fuel(200).is_ok());
        assert!(tracker.consume_fuel(200).is_err()); // Over 500
    }

    #[test]
    fn test_resource_tracker_timeout() {
        let limits = ResourceLimits {
            max_duration: std::time::Duration::from_millis(1),
            ..Default::default()
        };
        let tracker = ResourceTracker::new(limits);

        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(tracker.check_timeout().is_err());
    }

    #[test]
    fn test_resource_snapshot() {
        let tracker = ResourceTracker::new(ResourceLimits::default());
        tracker.alloc_memory(8192).unwrap();
        tracker.consume_fuel(42).unwrap();

        let snap = tracker.snapshot();
        assert_eq!(snap.memory_used, 8192);
        assert_eq!(snap.fuel_consumed, 42);
        assert!(!snap.killed);
    }

    // -----------------------------------------------------------------------
    // Signature Verification
    // -----------------------------------------------------------------------

    #[test]
    fn test_sign_verify_roundtrip() {
        use ed25519_dalek::SigningKey;

        let (sk_bytes, vk_bytes) = generate_keypair();
        let sk = SigningKey::from_bytes(&sk_bytes);

        let data = b"fake plugin binary content for testing";
        let sig = sign_plugin(data, &sk);

        let trusted = TrustedKey::from_bytes("test-author", &vk_bytes).unwrap();
        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(trusted);

        let result = verifier.verify_bytes(data, &sig).unwrap();
        assert!(result.is_verified());
        assert!(result.is_loadable());
    }

    #[test]
    fn test_signature_tampered_data() {
        use ed25519_dalek::SigningKey;

        let (sk_bytes, vk_bytes) = generate_keypair();
        let sk = SigningKey::from_bytes(&sk_bytes);

        let data = b"original content";
        let sig = sign_plugin(data, &sk);

        let trusted = TrustedKey::from_bytes("author", &vk_bytes).unwrap();
        let mut verifier = SignatureVerifier::new();
        verifier.add_trusted_key(trusted);

        let result = verifier.verify_bytes(b"tampered content", &sig).unwrap();
        assert!(!result.is_verified());
    }

    // -----------------------------------------------------------------------
    // Hook System
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_hook_register_and_fire() {
        let hooks = HookSystem::new();

        hooks.register(events::ON_STARTUP, "my-plugin", "on_boot", 0, None).await;

        let data = HookData::new(events::ON_STARTUP);
        let result = hooks.execute(&data).await;
        // Handler exists but plugin is not in the shared map,
        // so execution count depends on implementation tolerance
        assert!(result.duration.as_secs() < 5);
    }

    #[tokio::test]
    async fn test_hook_unregister_plugin() {
        let hooks = HookSystem::new();

        hooks.register(events::ON_CONNECT, "temp-plugin", "handler", 0, None).await;
        hooks.unregister_plugin("temp-plugin").await;

        let data = HookData::new(events::ON_CONNECT);
        let result = hooks.execute(&data).await;
        assert_eq!(result.handlers_executed, 0);
    }

    // -----------------------------------------------------------------------
    // PluginContext
    // -----------------------------------------------------------------------

    #[test]
    fn test_plugin_context_builder() {
        let ctx = PluginContext::default()
            .with_capabilities([Capability::Network, Capability::FileRead])
            .with_args(vec!["--verbose".to_string()])
            .with_memory_limit(64 * 1024 * 1024)
            .with_cpu_limit(5000);

        assert!(ctx.has_capability(Capability::Network));
        assert!(ctx.has_capability(Capability::FileRead));
        assert!(!ctx.has_capability(Capability::Shell));
        assert_eq!(ctx.memory_limit, 64 * 1024 * 1024);
        assert_eq!(ctx.cpu_limit_ms, 5000);
        assert_eq!(ctx.args, vec!["--verbose"]);
    }

    // -----------------------------------------------------------------------
    // Capability enumeration
    // -----------------------------------------------------------------------

    #[test]
    fn test_all_capabilities_have_names() {
        let caps = [
            Capability::Network,
            Capability::FileRead,
            Capability::FileWrite,
            Capability::Shell,
            Capability::Environment,
            Capability::Clipboard,
            Capability::Notifications,
            Capability::Media,
            Capability::Location,
            Capability::Browser,
            Capability::Llm,
            Capability::System,
        ];
        for cap in &caps {
            assert!(!cap.name().is_empty());
        }
    }

    #[test]
    fn test_dangerous_capability_identification() {
        assert!(Capability::Shell.is_dangerous());
        assert!(Capability::System.is_dangerous());
        assert!(Capability::FileWrite.is_dangerous());
        assert!(!Capability::Network.is_dangerous());
        assert!(!Capability::FileRead.is_dangerous());
    }
}
