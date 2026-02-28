use std::fs;
use std::path::{Path, PathBuf};

use glob::Pattern;

#[derive(Debug, Clone)]
struct Rule {
    pattern: Pattern,
    negated: bool,
    raw: String,
}

#[derive(Debug, Clone)]
pub struct ForgeIgnore {
    root: PathBuf,
    rules: Vec<Rule>,
}

impl ForgeIgnore {
    pub fn load(repo_root: &Path) -> Self {
        let mut rules = Vec::new();
        for default in [".forge/", ".git/", ".DS_Store", "Thumbs.db", "*.tmp"] {
            if let Ok(pattern) = Pattern::new(default) {
                rules.push(Rule {
                    pattern,
                    negated: false,
                    raw: default.to_string(),
                });
            }
        }

        let ignore_path = repo_root.join(".forgeignore");
        if let Ok(contents) = fs::read_to_string(ignore_path) {
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }

                let negated = trimmed.starts_with('!');
                let pattern_text = if negated { &trimmed[1..] } else { trimmed };
                if let Ok(pattern) = Pattern::new(pattern_text) {
                    rules.push(Rule {
                        pattern,
                        negated,
                        raw: pattern_text.to_string(),
                    });
                }
            }
        }

        Self {
            root: repo_root.to_path_buf(),
            rules,
        }
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        let rel = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();

        let mut ignored = false;
        for rule in &self.rules {
            let matches = rule.pattern.matches(&rel)
                || rule.pattern.matches(file_name)
                || (rule.raw.ends_with('/')
                    && rel
                        .strip_prefix(rule.raw.trim_end_matches('/'))
                        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/')))
                || (rel.ends_with('/') && rule.pattern.matches(&rel[..rel.len() - 1]));
            if matches {
                ignored = !rule.negated;
            }
        }
        ignored
    }
}
