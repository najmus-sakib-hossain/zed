//! Property-based tests for Plugin system compatibility.
//!
//! Tests:
//! - Property 17: Plugin Hook Filter Matching

use dx_compat_plugin::{
    Filter, ImportKind, Loader, OnLoadArgs, OnLoadResult, OnResolveArgs, OnResolveResult, Plugin,
    PluginBuilder, PluginRegistry,
};
use proptest::prelude::*;

/// Generate valid file extensions.
fn arb_extension() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("js".to_string()),
        Just("ts".to_string()),
        Just("jsx".to_string()),
        Just("tsx".to_string()),
        Just("css".to_string()),
        Just("json".to_string()),
        Just("txt".to_string()),
    ]
}

/// Generate valid file names.
fn arb_filename() -> impl Strategy<Value = String> {
    ("[a-z]{1,10}", arb_extension()).prop_map(|(name, ext)| format!("{}.{}", name, ext))
}

/// Generate valid namespace names.
fn arb_namespace() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("file".to_string()),
        Just("virtual".to_string()),
        Just("http".to_string()),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 17: Plugin Hook Filter Matching
    ///
    /// For any filter pattern:
    /// - Matching paths should trigger the handler
    /// - Non-matching paths should not trigger the handler
    /// - Namespace filters should be respected
    #[test]
    fn prop_plugin_filter_matching(
        filename in arb_filename(),
        namespace in arb_namespace(),
    ) {
        // Create a filter that matches .js files
        let filter = Filter::new(r"\.js$", None).unwrap();

        let is_js = filename.ends_with(".js");
        let matches = filter.matches(&filename, &namespace);

        prop_assert_eq!(matches, is_js, "Filter should match .js files only");
    }

    /// Property 17b: Namespace filtering works correctly
    #[test]
    fn prop_plugin_namespace_filter(
        filename in arb_filename(),
        filter_ns in arb_namespace(),
        actual_ns in arb_namespace(),
    ) {
        let filter = Filter::new(r".*", Some(&filter_ns)).unwrap();
        let matches = filter.matches(&filename, &actual_ns);

        prop_assert_eq!(
            matches,
            filter_ns == actual_ns,
            "Filter should only match when namespace matches"
        );
    }

    /// Property 17c: onLoad handlers are called for matching files
    #[test]
    fn prop_plugin_on_load_called(
        filename in arb_filename(),
    ) {
        let builder = PluginBuilder::new("test");

        // Register handler for .js files
        builder.on_load(r"\.js$", None, |args| {
            Some(OnLoadResult {
                contents: format!("// Processed: {}", args.path),
                loader: Loader::Js,
                resolve_dir: None,
            })
        }).unwrap();

        let args = OnLoadArgs {
            path: filename.clone(),
            namespace: "file".to_string(),
            suffix: String::new(),
        };

        let result = builder.run_on_load(&args);
        let is_js = filename.ends_with(".js");

        if is_js {
            prop_assert!(result.is_some(), "Handler should be called for .js files");
            prop_assert!(result.unwrap().contents.contains(&filename));
        } else {
            prop_assert!(result.is_none(), "Handler should not be called for non-.js files");
        }
    }

    /// Property 17d: onResolve handlers are called for matching imports
    #[test]
    fn prop_plugin_on_resolve_called(
        import_path in "[a-z]{1,10}",
        use_virtual_prefix in any::<bool>(),
    ) {
        let builder = PluginBuilder::new("test");

        // Register handler for virtual: imports
        builder.on_resolve(r"^virtual:", None, |args| {
            Some(OnResolveResult {
                path: args.path.replace("virtual:", "/virtual/"),
                namespace: Some("virtual".to_string()),
                external: false,
                side_effects: None,
            })
        }).unwrap();

        let path = if use_virtual_prefix {
            format!("virtual:{}", import_path)
        } else {
            import_path.clone()
        };

        let args = OnResolveArgs {
            path: path.clone(),
            importer: "index.js".to_string(),
            namespace: "file".to_string(),
            resolve_dir: "/project".to_string(),
            kind: ImportKind::Import,
        };

        let result = builder.run_on_resolve(&args);

        if use_virtual_prefix {
            prop_assert!(result.is_some(), "Handler should be called for virtual: imports");
            let res = result.unwrap();
            prop_assert_eq!(res.namespace, Some("virtual".to_string()));
        } else {
            prop_assert!(result.is_none(), "Handler should not be called for non-virtual imports");
        }
    }

    /// Property 17e: Multiple handlers are tried in order
    #[test]
    fn prop_plugin_handler_order(
        filename in arb_filename(),
    ) {
        let builder = PluginBuilder::new("test");

        // First handler: matches everything but returns None
        builder.on_load(r".*", None, |_| None).unwrap();

        // Second handler: matches .js and returns result
        builder.on_load(r"\.js$", None, |args| {
            Some(OnLoadResult {
                contents: format!("Second handler: {}", args.path),
                loader: Loader::Js,
                resolve_dir: None,
            })
        }).unwrap();

        let args = OnLoadArgs {
            path: filename.clone(),
            namespace: "file".to_string(),
            suffix: String::new(),
        };

        let result = builder.run_on_load(&args);

        if filename.ends_with(".js") {
            prop_assert!(result.is_some());
            prop_assert!(result.unwrap().contents.contains("Second handler"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_creation() {
        let plugin = Plugin::new("test-plugin", |builder| {
            builder.on_load(r"\.css$", None, |args| {
                Some(OnLoadResult {
                    contents: format!("/* {} */", args.path),
                    loader: Loader::Css,
                    resolve_dir: None,
                })
            })
        })
        .unwrap();

        assert_eq!(plugin.name(), "test-plugin");
    }

    #[test]
    fn test_plugin_registry() {
        let registry = PluginRegistry::new();

        let plugin = Plugin::new("css-plugin", |builder| {
            builder.on_load(r"\.css$", None, |args| {
                Some(OnLoadResult {
                    contents: format!("/* Processed: {} */", args.path),
                    loader: Loader::Css,
                    resolve_dir: None,
                })
            })
        })
        .unwrap();

        registry.register(plugin);
        assert_eq!(registry.plugin_count(), 1);

        let args = OnLoadArgs {
            path: "style.css".to_string(),
            namespace: "file".to_string(),
            suffix: String::new(),
        };

        let result = registry.run_on_load(&args);
        assert!(result.is_some());
        assert!(result.unwrap().contents.contains("style.css"));
    }

    #[test]
    fn test_virtual_module_plugin() {
        let plugin = Plugin::new("virtual-modules", |builder| {
            builder.on_resolve(r"^virtual:", None, |args| {
                Some(OnResolveResult {
                    path: args.path.clone(),
                    namespace: Some("virtual".to_string()),
                    external: false,
                    side_effects: Some(false),
                })
            })?;

            builder.on_load(r".*", Some("virtual"), |args| {
                let module_name = args.path.strip_prefix("virtual:").unwrap_or(&args.path);
                Some(OnLoadResult {
                    contents: format!("export default '{}';", module_name),
                    loader: Loader::Js,
                    resolve_dir: None,
                })
            })
        })
        .unwrap();

        // Test resolve
        let resolve_args = OnResolveArgs {
            path: "virtual:config".to_string(),
            importer: "index.js".to_string(),
            namespace: "file".to_string(),
            resolve_dir: "/project".to_string(),
            kind: ImportKind::Import,
        };

        let resolve_result = plugin.run_on_resolve(&resolve_args);
        assert!(resolve_result.is_some());
        assert_eq!(resolve_result.unwrap().namespace, Some("virtual".to_string()));

        // Test load
        let load_args = OnLoadArgs {
            path: "virtual:config".to_string(),
            namespace: "virtual".to_string(),
            suffix: String::new(),
        };

        let load_result = plugin.run_on_load(&load_args);
        assert!(load_result.is_some());
        assert!(load_result.unwrap().contents.contains("config"));
    }

    #[test]
    fn test_filter_complex_pattern() {
        // Match files in node_modules
        let filter = Filter::new(r"node_modules/", None).unwrap();
        assert!(filter.matches("node_modules/lodash/index.js", "file"));
        assert!(!filter.matches("src/index.js", "file"));

        // Match specific package
        let filter = Filter::new(r"node_modules/react", None).unwrap();
        assert!(filter.matches("node_modules/react/index.js", "file"));
        assert!(!filter.matches("node_modules/vue/index.js", "file"));
    }
}
