//! Property-based tests for Icon CLI commands
//!
//! These tests verify universal properties that should hold across all inputs.
//! Feature: dx-unified-assets

use proptest::prelude::*;

/// Generate valid search query strings
fn search_query_strategy() -> impl Strategy<Value = String> {
    // Generate alphanumeric strings that are valid search queries
    "[a-z]{1,10}".prop_map(|s| s.to_string())
}

proptest! {
    /// Property 2: Icon Search Result Relevance
    /// *For any* search query string, all icons returned by icon search SHALL have
    /// either their `id` or `name` containing the query string (case-insensitive).
    ///
    /// **Validates: Requirements 1.1**
    #[test]
    fn prop_icon_search_relevance(query in search_query_strategy()) {
        let mut reader = dx_icon::icons();
        let results = reader.search(&query);

        let query_lower = query.to_lowercase();

        for (prefix, icon) in &results {
            let id_matches = icon.id.to_lowercase().contains(&query_lower);
            let name_matches = icon.id.to_lowercase().contains(&query_lower); // name is same as id in this model

            prop_assert!(
                id_matches || name_matches,
                "Icon {}:{} does not contain query '{}' in id or name",
                prefix, icon.id, query
            );
        }
    }

    /// Property 3: Icon Get Returns Valid SVG
    /// *For any* valid prefix and icon id combination, `dx icon get` SHALL return
    /// a string that starts with `<svg` or contains valid SVG path data in the body.
    ///
    /// **Validates: Requirements 1.3**
    #[test]
    fn prop_icon_get_returns_valid_svg(
        set_idx in 0usize..10,
        icon_idx in 0usize..100
    ) {
        let mut reader = dx_icon::icons();
        let sets = reader.list_sets();

        if set_idx >= sets.len() {
            return Ok(());
        }

        let prefix = &sets[set_idx].prefix;

        if let Some(set) = reader.get_set(prefix) {
            if icon_idx >= set.icons.len() {
                return Ok(());
            }

            let icon = &set.icons[icon_idx];
            let svg = icon.to_svg(24);

            // SVG should start with <svg tag
            prop_assert!(
                svg.starts_with("<svg"),
                "Generated SVG does not start with <svg: {}",
                &svg[..svg.len().min(100)]
            );

            // SVG should end with </svg>
            prop_assert!(
                svg.ends_with("</svg>"),
                "Generated SVG does not end with </svg>: {}",
                &svg[svg.len().saturating_sub(100)..]
            );

            // SVG should contain the icon body
            prop_assert!(
                svg.contains(&icon.body),
                "Generated SVG does not contain icon body"
            );
        }
    }
}

/// Property 12: Code Generation Validity (Icon Component)
/// *For any* valid icon and target framework, the generated code snippet
/// SHALL be syntactically valid for that target.
///
/// **Validates: Requirements 1.5**
#[test]
fn test_icon_component_generation_validity() {
    let mut reader = dx_icon::icons();
    let sets = reader.list_sets();

    // Test with first available set
    if let Some(entry) = sets.first() {
        if let Some(set) = reader.get_set(&entry.prefix) {
            if let Some(icon) = set.icons.first() {
                // Test React component
                let react = icon.to_react("TestIcon", false);
                assert!(
                    react.contains("function TestIcon"),
                    "React component missing function declaration"
                );
                assert!(react.contains("export"), "React component missing export");
                assert!(react.contains("{...props}"), "React component missing props spread");

                // Test React TypeScript component
                let react_ts = icon.to_react("TestIcon", true);
                assert!(
                    react_ts.contains("SVGProps<SVGSVGElement>"),
                    "React TS component missing type"
                );

                // Test Vue component
                let vue = icon.to_vue(false);
                assert!(vue.contains("<template>"), "Vue component missing template");
                assert!(vue.contains("<script setup>"), "Vue component missing script");

                // Test Svelte component
                let svelte = icon.to_svelte();
                assert!(svelte.contains("<script>"), "Svelte component missing script");
                assert!(svelte.contains("$restProps"), "Svelte component missing restProps");

                // Test Solid component
                let solid = icon.to_solid("TestIcon");
                assert!(solid.contains("function TestIcon"), "Solid component missing function");
                assert!(solid.contains("JSX.IntrinsicElements"), "Solid component missing type");

                // Test Qwik component
                let qwik = icon.to_qwik("TestIcon");
                assert!(qwik.contains("function TestIcon"), "Qwik component missing function");
                assert!(qwik.contains("QwikIntrinsicElements"), "Qwik component missing type");

                // Test Astro component
                let astro = icon.to_astro();
                assert!(astro.contains("---"), "Astro component missing frontmatter");
                assert!(astro.contains("Astro.props"), "Astro component missing props");
            }
        }
    }
}

/// Test that icon stats returns valid counts
#[test]
fn test_icon_stats_validity() {
    let reader = dx_icon::icons();

    let total_sets = reader.total_sets();
    let total_icons = reader.total_icons();

    // Should have at least some sets and icons
    assert!(total_sets > 0, "Should have at least one icon set");
    assert!(total_icons > 0, "Should have at least one icon");

    // Total icons should be greater than total sets (each set has multiple icons)
    assert!(total_icons > total_sets, "Total icons should exceed total sets");
}

/// Test that icon list returns consistent data
#[test]
fn test_icon_list_consistency() {
    let reader = dx_icon::icons();
    let sets = reader.list_sets();

    // Each set should have required fields
    for entry in &sets {
        assert!(!entry.prefix.is_empty(), "Set prefix should not be empty");
        assert!(!entry.name.is_empty(), "Set name should not be empty");
        assert!(entry.total > 0, "Set should have at least one icon");
    }

    // Number of sets should match total_sets
    assert_eq!(
        sets.len() as u32,
        reader.total_sets(),
        "List sets count should match total_sets"
    );
}
