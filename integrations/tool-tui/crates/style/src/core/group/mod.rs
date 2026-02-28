//! Group Registry module
//!
//! Manages auto-grouped classnames and their CSS generation.
//! Provides the `GroupRegistry` for tracking group definitions,
//! generating CSS for grouped classes, and managing group aliases.

use ahash::{AHashMap, AHashSet};

use crate::parser::GroupEvent;

use super::engine::StyleEngine;

/// Generate a simple ISO 8601 timestamp without external dependencies.
#[allow(dead_code)]
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();

    let secs = duration.as_secs();

    // Calculate date components (simplified, doesn't handle leap seconds)
    let days = secs / 86400;
    let time_secs = secs % 86400;

    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Calculate year, month, day from days since epoch (1970-01-01)
    let mut year = 1970i32;
    let mut remaining_days = days as i32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months: [i32; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for days_in_month in days_in_months.iter() {
        if remaining_days < *days_in_month {
            break;
        }
        remaining_days -= days_in_month;
        month += 1;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

#[allow(dead_code)]
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[derive(Debug, Default, Clone)]
pub struct GroupDefinition {
    pub utilities: Vec<String>,
    pub allow_extend: bool,
    pub raw_tokens: Vec<String>,
    pub dev_tokens: Vec<String>,
}

#[derive(Debug, Default, Clone)]
pub struct GroupRegistry {
    definitions: AHashMap<String, GroupDefinition>,
    internal_tokens: AHashSet<String>,
    utility_members: AHashSet<String>,
    cached_css: AHashMap<String, String>,
    dev_selectors: AHashMap<String, String>,
    /// Track original classnames for stability across rebuilds
    /// Maps: hash of utilities -> original classname
    #[allow(dead_code)]
    stable_classnames: AHashMap<u64, String>,
}

#[allow(dead_code)]
impl GroupRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a group definition to the registry.
    pub fn add_definition(&mut self, alias: String, definition: GroupDefinition) {
        // Track utility members
        for util in &definition.utilities {
            self.utility_members.insert(util.clone());
        }
        self.definitions.insert(alias, definition);
    }

    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    pub fn definitions(&self) -> impl Iterator<Item = (&String, &GroupDefinition)> {
        self.definitions.iter()
    }

    pub fn set_dev_selectors(&mut self, selectors: AHashMap<String, String>) {
        // Note: selectors.is_empty() check removed - placeholder for future logging
        let old_selectors = std::mem::replace(&mut self.dev_selectors, selectors);

        if old_selectors.is_empty() {
            return;
        }

        let removed: Vec<String> = old_selectors
            .keys()
            .filter(|k| !self.dev_selectors.contains_key(*k))
            .cloned()
            .collect();
        for alias in removed {
            self.cached_css.remove(&alias);
        }
    }

    pub fn is_internal_token(&self, class: &str) -> bool {
        self.internal_tokens.contains(class)
    }

    pub fn merge_preserve(&mut self, prev: &GroupRegistry) {
        for (name, def) in prev.definitions.iter() {
            self.definitions.entry(name.clone()).or_insert_with(|| def.clone());
        }

        let mut current_alias_names: AHashSet<String> = AHashSet::default();
        for (name, _) in self.definitions.iter() {
            current_alias_names.insert(name.clone());
        }

        let mut current_defs_norm: Vec<(String, AHashSet<String>)> = Vec::new();
        for (name, def) in self.definitions.iter() {
            let mut set: AHashSet<String> = AHashSet::default();
            for u in &def.utilities {
                if u.is_empty() {
                    continue;
                }
                if u.contains('@') {
                    continue;
                }
                if current_alias_names.contains(u) {
                    continue;
                }
                set.insert(u.clone());
            }
            if !set.is_empty() {
                current_defs_norm.push((name.clone(), set));
            }
        }
        for (k, v) in prev.cached_css.iter() {
            if let Some(current_def) = self.definitions.get(k) {
                if let Some(prev_def) = prev.definitions.get(k) {
                    if current_def.utilities == prev_def.utilities {
                        self.cached_css.entry(k.clone()).or_insert_with(|| v.clone());
                    }
                    // Note: empty else removed - utilities differ, skip caching
                    continue;
                }
            }

            if let Some(prev_def) = prev.definitions.get(k) {
                let mut prev_set: AHashSet<String> = AHashSet::default();
                for u in &prev_def.utilities {
                    if u.is_empty() {
                        continue;
                    }
                    if u.contains('@') {
                        continue;
                    }
                    if current_alias_names.contains(u) {
                        continue;
                    }
                    prev_set.insert(u.clone());
                }
                if !prev_set.is_empty() {
                    let mut best_score = 0f64;
                    let mut best_alias: Option<String> = None;
                    for (cand_alias, cand_set) in &current_defs_norm {
                        let inter = prev_set.iter().filter(|x| cand_set.contains(*x)).count();
                        let uni = prev_set.len() + cand_set.len() - inter;
                        if uni == 0 {
                            continue;
                        }
                        let score = (inter as f64) / (uni as f64);
                        if score > best_score {
                            best_score = score;
                            best_alias = Some(cand_alias.clone());
                        }
                    }
                    let threshold: f64 = std::env::var("DX_GROUP_RENAME_SIMILARITY")
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0.6);
                    if let Some(new_alias) = best_alias {
                        if best_score >= threshold {
                            self.cached_css.entry(new_alias.clone()).or_insert_with(|| v.clone());
                            continue;
                        }
                    }
                }
            }

            self.cached_css.entry(k.clone()).or_insert_with(|| v.clone());
        }
        for tok in prev.utility_members.iter() {
            self.utility_members.insert(tok.clone());
        }
    }

    pub fn to_dump(&self) -> super::super::cache::GroupDump {
        self.to_dump_with_metadata(None)
    }

    /// Convert to GroupDump with optional timestamp and source tracking.
    /// Format follows DX LLM format:
    /// ```text
    /// # .dx/style/groups.dx
    /// v|1
    /// ts|2024-12-28T10:00:00Z
    ///
    /// #g(alias|classes|source)
    /// dxg-a1b2c|flex items-center p-4|src/app.html:15
    /// ```
    pub fn to_dump_with_metadata(&self, timestamp: Option<&str>) -> super::super::cache::GroupDump {
        use std::collections::BTreeMap;
        let mut defs = BTreeMap::new();
        for (k, v) in self.definitions.iter() {
            defs.insert(
                k.clone(),
                super::super::cache::GroupDefDump {
                    utilities: v.utilities.clone(),
                    allow_extend: v.allow_extend,
                    raw_tokens: v.raw_tokens.clone(),
                    dev_tokens: v.dev_tokens.clone(),
                },
            );
        }
        let mut css_map = BTreeMap::new();
        for (k, v) in self.cached_css.iter() {
            css_map.insert(k.clone(), v.clone());
        }

        // Add timestamp metadata if provided
        if let Some(ts) = timestamp {
            css_map.insert("__meta_timestamp__".to_string(), ts.to_string());
        }

        super::super::cache::GroupDump {
            definitions: defs,
            cached_css: css_map,
        }
    }

    /// Serialize to DX LLM format string.
    pub fn to_dx_llm_format(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str("v|1\n");

        // Timestamp
        let timestamp = chrono_lite_timestamp();
        output.push_str(&format!("ts|{}\n\n", timestamp));

        // Group definitions header
        output.push_str("#g(alias|classes|allow_extend)\n");

        // Write each group definition
        for (alias, def) in self.definitions.iter() {
            let classes = def.utilities.join(" ");
            let allow_extend = if def.allow_extend { "1" } else { "0" };
            output.push_str(&format!("{}|{}|{}\n", alias, classes, allow_extend));
        }

        // Cached CSS section
        if !self.cached_css.is_empty() {
            output.push_str("\n#css(alias|css)\n");
            for (alias, css) in self.cached_css.iter() {
                // Escape newlines in CSS
                let escaped_css = css.replace('\n', "\\n");
                output.push_str(&format!("{}|{}\n", alias, escaped_css));
            }
        }

        output
    }

    /// Parse from DX LLM format string.
    pub fn from_dx_llm_format(input: &str) -> Result<Self, String> {
        let mut registry = GroupRegistry::new();
        let mut in_groups = false;
        let mut in_css = false;

        for line in input.lines() {
            let line = line.trim();

            // Skip empty lines and version/timestamp
            if line.is_empty() || line.starts_with("v|") || line.starts_with("ts|") {
                continue;
            }

            // Section headers
            if line.starts_with("#g(") {
                in_groups = true;
                in_css = false;
                continue;
            }
            if line.starts_with("#css(") {
                in_groups = false;
                in_css = true;
                continue;
            }

            // Parse group definitions
            if in_groups {
                let parts: Vec<&str> = line.splitn(3, '|').collect();
                if parts.len() >= 2 {
                    let alias = parts[0].to_string();
                    let classes: Vec<String> =
                        parts[1].split_whitespace().map(|s| s.to_string()).collect();
                    let allow_extend = parts.get(2).map(|s| *s == "1").unwrap_or(false);

                    // Track stable classname
                    let hash = registry.compute_utilities_hash(&classes);
                    registry.stable_classnames.insert(hash, alias.clone());

                    for util in &classes {
                        registry.utility_members.insert(util.clone());
                    }

                    registry.definitions.insert(
                        alias,
                        GroupDefinition {
                            utilities: classes,
                            allow_extend,
                            raw_tokens: Vec::new(),
                            dev_tokens: Vec::new(),
                        },
                    );
                }
            }

            // Parse cached CSS
            if in_css {
                if let Some(pipe_idx) = line.find('|') {
                    let alias = line[..pipe_idx].to_string();
                    let css = line[pipe_idx + 1..].replace("\\n", "\n");
                    registry.cached_css.insert(alias, css);
                }
            }
        }

        Ok(registry)
    }

    /// Compute a hash for a set of utilities (for stable classname tracking).
    fn compute_utilities_hash(&self, utilities: &[String]) -> u64 {
        use std::hash::Hasher;
        let mut sorted = utilities.to_vec();
        sorted.sort();
        let mut hasher = ahash::AHasher::default();
        for util in sorted {
            hasher.write(util.as_bytes());
            hasher.write_u8(0); // separator
        }
        hasher.finish()
    }

    /// Get a stable classname for a set of utilities.
    /// If the utilities were previously registered, returns the original classname.
    /// Otherwise returns None.
    pub fn get_stable_classname(&self, utilities: &[String]) -> Option<&String> {
        let hash = self.compute_utilities_hash(utilities);
        self.stable_classnames.get(&hash)
    }

    /// Register a stable classname for a set of utilities.
    pub fn register_stable_classname(&mut self, alias: &str, utilities: &[String]) {
        let hash = self.compute_utilities_hash(utilities);
        self.stable_classnames.insert(hash, alias.to_string());
    }

    /// Update a group's CSS while preserving its classname.
    /// This is used when the underlying classes change but we want to keep the same alias.
    pub fn update_group_css(&mut self, alias: &str, new_utilities: Vec<String>) {
        if let Some(def) = self.definitions.get_mut(alias) {
            // Remove old utility members
            for util in &def.utilities {
                self.utility_members.remove(util);
            }

            // Update utilities
            def.utilities = new_utilities.clone();

            // Add new utility members
            for util in &new_utilities {
                self.utility_members.insert(util.clone());
            }

            // Invalidate cached CSS (will be regenerated)
            self.cached_css.remove(alias);

            // Update stable classname mapping
            let hash = self.compute_utilities_hash(&new_utilities);
            self.stable_classnames.insert(hash, alias.to_string());
        }
    }

    pub fn from_dump(dump: &super::super::cache::GroupDump) -> Self {
        let mut registry = GroupRegistry::new();
        for (k, v) in dump.definitions.iter() {
            registry.definitions.insert(
                k.clone(),
                GroupDefinition {
                    utilities: v.utilities.clone(),
                    allow_extend: v.allow_extend,
                    raw_tokens: v.raw_tokens.clone(),
                    dev_tokens: v.dev_tokens.clone(),
                },
            );
            for util in &v.utilities {
                registry.utility_members.insert(util.clone());
            }
        }
        for (k, v) in dump.cached_css.iter() {
            registry.cached_css.insert(k.clone(), v.clone());
        }
        registry
    }

    pub fn analyze(
        events: &[GroupEvent],
        classes: &mut AHashSet<String>,
        engine: Option<&StyleEngine>,
    ) -> Self {
        let mut registry = GroupRegistry::default();
        if events.is_empty() {
            return registry;
        }

        let mut known_prefixes: AHashSet<String> = AHashSet::default();
        if let Some(engine) = engine {
            known_prefixes.extend(engine.screens.keys().cloned());
            known_prefixes.extend(engine.states.keys().cloned());
            known_prefixes.extend(engine.container_queries.keys().cloned());
        }

        for event in events {
            if event.stack.is_empty() {
                continue;
            }
            let mut alias_idx: Option<usize> = None;
            for (idx, seg) in event.stack.iter().enumerate() {
                if known_prefixes.contains(seg) {
                    continue;
                }
                alias_idx = Some(idx);
                break;
            }
            let Some(idx) = alias_idx else {
                continue;
            };
            let alias_name = event.stack[idx].clone();
            if alias_name.is_empty() {
                continue;
            }
            let recognized_prefixes = &event.stack[..idx];
            let actual_class = if recognized_prefixes.is_empty() {
                event.token.clone()
            } else {
                build_prefixed_class(recognized_prefixes, &event.token)
            };

            let entry =
                registry
                    .definitions
                    .entry(alias_name.clone())
                    .or_insert_with(|| GroupDefinition {
                        utilities: Vec::new(),
                        allow_extend: false,
                        raw_tokens: Vec::new(),
                        dev_tokens: Vec::new(),
                    });
            if !entry.utilities.contains(&actual_class) {
                entry.utilities.push(actual_class.clone());
            }
            registry.utility_members.insert(actual_class.clone());
            entry.raw_tokens.push(event.full_class.clone());
            if !entry.dev_tokens.contains(&event.token) {
                entry.dev_tokens.push(event.token.clone());
            }
            if event.had_plus {
                entry.allow_extend = true;
            }

            registry.internal_tokens.insert(event.full_class.clone());
            classes.insert(alias_name);
            classes.insert(actual_class);
        }

        for token in registry.internal_tokens.iter() {
            classes.remove(token);
        }

        registry
    }

    pub fn remove_utility_members_from(&self, classes: &mut AHashSet<String>) {
        for util in self.utility_members.iter() {
            classes.remove(util);
        }
    }

    pub fn is_util_member(&self, class: &str) -> bool {
        self.utility_members.contains(class)
    }

    pub fn generate_css_for<'a>(
        &'a mut self,
        class: &str,
        engine: &StyleEngine,
    ) -> Option<&'a str> {
        if self.internal_tokens.contains(class) {
            return None;
        }
        let utilities = match self.definitions.get(class) {
            Some(def) => def.utilities.clone(),
            None => return None,
        };

        let mut visited: AHashSet<String> = AHashSet::default();
        let mut flattened: Vec<String> = Vec::new();
        let mut seen: AHashSet<String> = AHashSet::default();
        for util in &utilities {
            collect_final_classes(self, util, &mut visited, &mut flattened, &mut seen);
        }

        if flattened.is_empty() {
            return None;
        }

        let alias_selector = make_selector(class);
        let dev_selector = self
            .dev_selectors
            .get(class)
            .and_then(|raw| parse_grouped_selector(raw, alias_selector.as_str()));
        let combined_selector =
            dev_selector.as_ref().map(|dev| format!("{},{}", alias_selector, dev));
        let mut simple_bodies: Vec<String> = Vec::new();
        let mut extra_css = String::new();
        let mut missing_utils: Vec<String> = Vec::new();
        for util in flattened {
            if let Some(mut css) = engine.css_for_class(&util) {
                rewrite_selector(&mut css, &util, &alias_selector);
                let trimmed_css = css.trim();
                let mut handled_simple = false;
                if let Some(open_idx) = trimmed_css.find('{') {
                    if trimmed_css.ends_with('}') {
                        let selector = trimmed_css[..open_idx].trim();
                        let body = trimmed_css[open_idx + 1..trimmed_css.len() - 1].trim();
                        if selector == alias_selector
                            && !selector.contains(',')
                            && !body.contains('{')
                            && !body.contains('}')
                        {
                            simple_bodies.push(body.to_string());
                            handled_simple = true;
                        }
                    }
                }
                if !handled_simple {
                    if !extra_css.is_empty() && !extra_css.ends_with('\n') {
                        extra_css.push('\n');
                    }
                    extra_css.push_str(trimmed_css);
                    if !trimmed_css.ends_with('\n') {
                        extra_css.push('\n');
                    }
                }
            } else {
                missing_utils.push(util.clone());
            }
        }

        let mut simple_block = String::new();
        if !simple_bodies.is_empty() {
            let selector_output = combined_selector.as_deref().unwrap_or(alias_selector.as_str());
            simple_block.push_str(selector_output);
            simple_block.push_str(" {\n");
            for body in &simple_bodies {
                for line in body.lines() {
                    let trimmed_line = line.trim();
                    if trimmed_line.is_empty() {
                        continue;
                    }
                    simple_block.push_str("  ");
                    simple_block.push_str(trimmed_line);
                    if !trimmed_line.ends_with(';') && !trimmed_line.ends_with('}') {
                        simple_block.push(';');
                    }
                    simple_block.push('\n');
                }
            }
            simple_block.push_str("}\n");
        }

        let mut accumulated = String::new();
        if !simple_block.is_empty() {
            accumulated.push_str(&simple_block);
        }
        if !extra_css.is_empty() {
            if !accumulated.is_empty() && !accumulated.ends_with('\n') {
                accumulated.push('\n');
            }
            accumulated.push_str(&extra_css);
            if let Some(ref dev_sel) = dev_selector {
                if !extra_css.ends_with('\n') {
                    accumulated.push('\n');
                }
                accumulated.push_str(&extra_css.replace(alias_selector.as_str(), dev_sel));
            }
        }

        if accumulated.trim().is_empty() {
            // Note: missing_utils check removed - placeholder for future logging
            return None;
        }

        let existing_clone = self.cached_css.get(class).cloned();
        if let Some(ref old) = existing_clone {
            if old == &accumulated {
                return Some(self.cached_css.get(class).unwrap().as_str());
            }
        }

        self.cached_css.insert(class.to_string(), accumulated);
        Some(self.cached_css.get(class).unwrap().as_str())
    }
}

fn build_prefixed_class(prefixes: &[String], token: &str) -> String {
    if prefixes.is_empty() {
        return token.to_string();
    }
    let total_len = prefixes.iter().map(|p| p.len() + 1).sum::<usize>() + token.len();
    let mut out = String::with_capacity(total_len);
    for (idx, prefix) in prefixes.iter().enumerate() {
        if idx > 0 {
            out.push(':');
        }
        out.push_str(prefix);
    }
    out.push(':');
    out.push_str(token);
    out
}

fn collect_final_classes(
    registry: &GroupRegistry,
    util: &str,
    visited: &mut AHashSet<String>,
    out: &mut Vec<String>,
    seen: &mut AHashSet<String>,
) {
    if seen.contains(util) {
        return;
    }
    if visited.contains(util) {
        return;
    }
    if let Some(def) = registry.definitions.get(util) {
        visited.insert(util.to_string());
        for child in &def.utilities {
            collect_final_classes(registry, child, visited, out, seen);
        }
        visited.remove(util);
    } else if seen.insert(util.to_string()) {
        out.push(util.to_string());
    }
}

fn make_selector(class: &str) -> String {
    let mut escaped = String::new();
    cssparser::serialize_identifier(class, &mut escaped).unwrap();
    format!(".{}", escaped)
}

fn rewrite_selector(css: &mut String, original: &str, alias_selector: &str) {
    let mut escaped_original = String::new();
    cssparser::serialize_identifier(original, &mut escaped_original).unwrap();
    let original_selector = format!(".{}", escaped_original);
    if original_selector == alias_selector {
        return;
    }
    *css = css.replace(&original_selector, alias_selector);
}

fn parse_grouped_selector(raw: &str, alias_selector: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || !raw.starts_with('@') {
        return None;
    }
    let open_idx = raw.find('(')?;
    let alias_part = raw[1..open_idx].trim();
    if alias_part.is_empty() {
        return None;
    }
    let inner = raw[open_idx + 1..].trim().strip_suffix(')')?;

    let mut parsed_alias = String::new();
    cssparser::serialize_identifier(alias_part, &mut parsed_alias).ok()?;
    let expected_alias = alias_selector.strip_prefix('.')?;
    if parsed_alias != expected_alias {
        return None;
    }

    let mut inner_sanitized = String::new();
    let mut first = true;
    for token in inner.split_whitespace() {
        if token.is_empty() {
            continue;
        }
        if !first {
            inner_sanitized.push(' ');
        }
        first = false;
        let mut sanitized = String::new();
        cssparser::serialize_identifier(token, &mut sanitized).ok()?;
        inner_sanitized.push_str(&sanitized);
    }
    if inner_sanitized.is_empty() {
        return None;
    }

    let mut class_name = String::new();
    class_name.push('@');
    class_name.push_str(&parsed_alias);
    class_name.push('(');
    class_name.push_str(&inner_sanitized);
    class_name.push(')');

    let mut escaped = String::with_capacity(class_name.len() + 1);
    escaped.push('.');
    cssparser::serialize_identifier(&class_name, &mut escaped).ok()?;
    Some(escaped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::extract_classes_fast;
    use ahash::AHashMap;
    use std::io::Write;

    #[test]
    fn alias_generates_combined_css() {
        let html = br#"<div dx-text="card(bg-red-500 h-50)"></div>"#;
        let extracted = extract_classes_fast(html, 0);
        let mut classes = extracted.classes.clone();
        let temp_path = std::env::temp_dir().join("dx_style_test_style.bin");
        {
            let mut f = std::fs::File::create(&temp_path).unwrap();
            f.write_all(&vec![0u8; 4096]).unwrap();
        }
        unsafe {
            std::env::set_var("DX_STYLE_BIN", &temp_path);
        }
        let mut engine = crate::core::engine::StyleEngine::empty();
        unsafe {
            std::env::remove_var("DX_STYLE_BIN");
        }
        engine
            .precompiled
            .insert("bg-red-500".to_string(), "background-color: red;".to_string());
        engine.precompiled.insert("h-50".to_string(), "height: 12.5rem;".to_string());
        let registry = GroupRegistry::analyze(&extracted.group_events, &mut classes, Some(&engine));
        assert!(classes.contains("card"));
        let mut registry = registry;
        let mut selectors = AHashMap::default();
        selectors.insert("card".to_string(), "@card(bg-red-500 h-50)".to_string());
        registry.set_dev_selectors(selectors);
        let css = registry.generate_css_for("card", &engine).expect("css");
        assert!(css.contains(".card"));
        assert!(css.contains(".card,.\\@card\\(bg-red-500\\ h-50\\)"));
        assert!(css.contains("background-color: red"));
        assert!(css.contains("height"));
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_dx_llm_format_roundtrip() {
        let mut registry = GroupRegistry::new();

        // Add some definitions
        registry.definitions.insert(
            "dxg-abc12".to_string(),
            GroupDefinition {
                utilities: vec!["flex".to_string(), "items-center".to_string()],
                allow_extend: false,
                raw_tokens: Vec::new(),
                dev_tokens: Vec::new(),
            },
        );
        registry.definitions.insert(
            "dxg-xyz99".to_string(),
            GroupDefinition {
                utilities: vec![
                    "bg-white".to_string(),
                    "rounded".to_string(),
                    "shadow".to_string(),
                ],
                allow_extend: true,
                raw_tokens: Vec::new(),
                dev_tokens: Vec::new(),
            },
        );

        // Add cached CSS
        registry.cached_css.insert(
            "dxg-abc12".to_string(),
            ".dxg-abc12 { display: flex; align-items: center; }".to_string(),
        );

        // Serialize to DX LLM format
        let llm_format = registry.to_dx_llm_format();

        // Verify format structure
        assert!(llm_format.contains("v|1"));
        assert!(llm_format.contains("ts|"));
        assert!(llm_format.contains("#g(alias|classes|allow_extend)"));
        assert!(llm_format.contains("dxg-abc12|flex items-center|0"));
        assert!(llm_format.contains("dxg-xyz99|bg-white rounded shadow|1"));
        assert!(llm_format.contains("#css(alias|css)"));

        // Parse back
        let parsed = GroupRegistry::from_dx_llm_format(&llm_format).unwrap();

        // Verify definitions
        assert_eq!(parsed.definitions.len(), 2);

        let def1 = parsed.definitions.get("dxg-abc12").unwrap();
        assert_eq!(def1.utilities, vec!["flex", "items-center"]);
        assert!(!def1.allow_extend);

        let def2 = parsed.definitions.get("dxg-xyz99").unwrap();
        assert_eq!(def2.utilities, vec!["bg-white", "rounded", "shadow"]);
        assert!(def2.allow_extend);

        // Verify cached CSS
        assert!(parsed.cached_css.contains_key("dxg-abc12"));
    }

    #[test]
    fn test_stable_classname_tracking() {
        let mut registry = GroupRegistry::new();

        let utilities = vec!["flex".to_string(), "items-center".to_string()];

        // Register a stable classname
        registry.register_stable_classname("dxg-stable", &utilities);

        // Should retrieve the same classname
        assert_eq!(registry.get_stable_classname(&utilities), Some(&"dxg-stable".to_string()));

        // Different order should still match (sorted internally)
        let utilities_reordered = vec!["items-center".to_string(), "flex".to_string()];
        assert_eq!(
            registry.get_stable_classname(&utilities_reordered),
            Some(&"dxg-stable".to_string())
        );

        // Different utilities should not match
        let different_utilities = vec!["flex".to_string(), "justify-center".to_string()];
        assert_eq!(registry.get_stable_classname(&different_utilities), None);
    }

    #[test]
    fn test_update_group_css() {
        let mut registry = GroupRegistry::new();

        // Add initial definition
        registry.definitions.insert(
            "card".to_string(),
            GroupDefinition {
                utilities: vec!["bg-white".to_string(), "rounded".to_string()],
                allow_extend: false,
                raw_tokens: Vec::new(),
                dev_tokens: Vec::new(),
            },
        );
        registry.utility_members.insert("bg-white".to_string());
        registry.utility_members.insert("rounded".to_string());
        registry.cached_css.insert("card".to_string(), ".card { ... }".to_string());

        // Update the group
        let new_utilities = vec![
            "bg-gray-100".to_string(),
            "rounded-lg".to_string(),
            "shadow".to_string(),
        ];
        registry.update_group_css("card", new_utilities.clone());

        // Verify update
        let def = registry.definitions.get("card").unwrap();
        assert_eq!(def.utilities, new_utilities);

        // Old utilities should be removed
        assert!(!registry.utility_members.contains("bg-white"));
        assert!(!registry.utility_members.contains("rounded"));

        // New utilities should be added
        assert!(registry.utility_members.contains("bg-gray-100"));
        assert!(registry.utility_members.contains("rounded-lg"));
        assert!(registry.utility_members.contains("shadow"));

        // Cached CSS should be invalidated
        assert!(!registry.cached_css.contains_key("card"));
    }

    #[test]
    fn test_chrono_lite_timestamp() {
        let ts = chrono_lite_timestamp();

        // Should be in ISO 8601 format
        assert!(ts.contains("T"));
        assert!(ts.ends_with("Z"));
        assert_eq!(ts.len(), 20); // YYYY-MM-DDTHH:MM:SSZ

        // Year should be reasonable
        let year: i32 = ts[0..4].parse().unwrap();
        assert!(year >= 2024);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // Generate valid utility class names
    fn arb_utility_class() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{1,15}"
    }

    // Generate a vector of utility classes
    fn arb_utilities() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(arb_utility_class(), 2..8)
    }

    // Generate a group alias name
    fn arb_alias() -> impl Strategy<Value = String> {
        "dxg-[a-z0-9]{5}"
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-production-ready, Property 13: Group Registry Stability
        /// *For any* group whose underlying classes change, the Group_Registry SHALL
        /// preserve the original classname while updating the CSS.
        /// **Validates: Requirements 6.3**
        #[test]
        fn prop_group_registry_stability(
            alias in arb_alias(),
            original_utilities in arb_utilities(),
            new_utilities in arb_utilities()
        ) {
            let mut registry = GroupRegistry::new();

            // Register original group
            registry.definitions.insert(
                alias.clone(),
                GroupDefinition {
                    utilities: original_utilities.clone(),
                    allow_extend: false,
                    raw_tokens: Vec::new(),
                    dev_tokens: Vec::new(),
                },
            );

            // Register stable classname
            registry.register_stable_classname(&alias, &original_utilities);

            // Update the group with new utilities
            registry.update_group_css(&alias, new_utilities.clone());

            // The alias should still exist
            prop_assert!(
                registry.definitions.contains_key(&alias),
                "Alias '{}' should still exist after update",
                alias
            );

            // The utilities should be updated
            let def = registry.definitions.get(&alias).unwrap();
            prop_assert_eq!(
                &def.utilities, &new_utilities,
                "Utilities should be updated"
            );

            // The stable classname should now map to the new utilities
            let stable = registry.get_stable_classname(&new_utilities);
            prop_assert_eq!(
                stable, Some(&alias),
                "Stable classname should map to new utilities"
            );
        }

        /// Property: DX LLM format round-trip preserves all data
        #[test]
        fn prop_dx_llm_format_roundtrip(
            aliases in prop::collection::vec(arb_alias(), 1..5),
            utilities_list in prop::collection::vec(arb_utilities(), 1..5)
        ) {
            let mut registry = GroupRegistry::new();

            // Add definitions (zip aliases with utilities)
            for (alias, utilities) in aliases.iter().zip(utilities_list.iter()) {
                registry.definitions.insert(
                    alias.clone(),
                    GroupDefinition {
                        utilities: utilities.clone(),
                        allow_extend: false,
                        raw_tokens: Vec::new(),
                        dev_tokens: Vec::new(),
                    },
                );
            }

            // Serialize to DX LLM format
            let llm_format = registry.to_dx_llm_format();

            // Parse back
            let parsed = GroupRegistry::from_dx_llm_format(&llm_format).unwrap();

            // Verify all definitions are preserved
            prop_assert_eq!(
                parsed.definitions.len(),
                registry.definitions.len(),
                "Definition count should match"
            );

            for (alias, def) in registry.definitions.iter() {
                let parsed_def = parsed.definitions.get(alias);
                prop_assert!(
                    parsed_def.is_some(),
                    "Alias '{}' should exist in parsed registry",
                    alias
                );

                let parsed_def = parsed_def.unwrap();
                prop_assert_eq!(
                    &parsed_def.utilities, &def.utilities,
                    "Utilities for '{}' should match",
                    alias
                );
            }
        }

        /// Property: Stable classname lookup is order-independent
        #[test]
        fn prop_stable_classname_order_independent(utilities in arb_utilities()) {
            let mut registry = GroupRegistry::new();
            let alias = "dxg-test1".to_string();

            // Register with original order
            registry.register_stable_classname(&alias, &utilities);

            // Lookup with reversed order
            let mut reversed = utilities.clone();
            reversed.reverse();

            let result = registry.get_stable_classname(&reversed);
            prop_assert_eq!(
                result, Some(&alias),
                "Lookup should be order-independent"
            );
        }
    }
}
