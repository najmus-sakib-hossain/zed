//! Unit tests for framework configuration components

#[cfg(test)]
mod tests {
    use dx_check::framework_config::{FrameworkConfig, FrameworkConfigManager, FrameworkSetting};
    use dx_check::project::Framework;
    use std::collections::HashMap;

    #[test]
    fn test_framework_config_manager_initialization() {
        let manager = FrameworkConfigManager::new();

        // Should have default configs for all frameworks
        assert!(manager.get_config(Framework::React).is_none());
        assert!(manager.get_config(Framework::Next).is_none());
    }

    #[test]
    fn test_framework_setting_types() {
        let mut settings = HashMap::new();
        settings.insert("bool_setting".to_string(), FrameworkSetting::Boolean(true));
        settings.insert("num_setting".to_string(), FrameworkSetting::Number(42));
        settings.insert("str_setting".to_string(), FrameworkSetting::String("value".to_string()));

        assert_eq!(settings.len(), 3);
    }

    #[test]
    fn test_framework_config_structure() {
        let config = FrameworkConfig {
            framework: Framework::React,
            version: Some("18.0.0".to_string()),
            settings: HashMap::new(),
            enabled_rules: vec!["react-hooks/rules-of-hooks".to_string()],
            disabled_rules: vec![],
            rule_overrides: HashMap::new(),
        };

        assert_eq!(config.framework, Framework::React);
        assert_eq!(config.version, Some("18.0.0".to_string()));
        assert_eq!(config.enabled_rules.len(), 1);
    }
}
