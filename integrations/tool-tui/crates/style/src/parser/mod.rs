//! HTML Parser module
//!
//! Provides fast HTML parsing for CSS class extraction using SIMD-accelerated
//! string searching. Supports grouping syntax, incremental parsing, and
//! automatic duplicate class detection.

use ahash::{AHashMap, AHashSet, AHasher};
use memchr::{memchr, memmem::Finder};
use smallvec::SmallVec;
use std::hash::Hasher;
use std::ops::Range;

pub mod arbitrary;
pub mod incremental;
pub mod optimized;
pub mod simd;

// Re-export incremental parser
pub use incremental::IncrementalParser;

#[derive(Debug, Clone)]
pub struct GroupEvent {
    pub stack: Vec<String>,
    pub token: String,
    pub had_plus: bool,
    pub full_class: String,
}

#[derive(Debug, Default, Clone)]
pub struct GroupCollector {
    events: Vec<GroupEvent>,
}

impl GroupCollector {
    #[inline]
    fn record(&mut self, stack: &[String], token: &str, had_plus: bool, full_class: &str) {
        if stack.is_empty() {
            return;
        }
        self.events.push(GroupEvent {
            stack: stack.to_vec(),
            token: token.to_string(),
            had_plus,
            full_class: full_class.to_string(),
        });
    }

    pub fn into_events(self) -> Vec<GroupEvent> {
        self.events
    }
}

pub struct ExtractedClasses {
    pub classes: AHashSet<String>,
    pub group_events: Vec<GroupEvent>,
}

#[derive(Debug, Clone)]
pub struct AutoGroupInfo {
    pub alias: String,
    pub classes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AutoGroupRewrite {
    pub html: Vec<u8>,
    pub groups: Vec<AutoGroupInfo>,
}

#[inline]
fn base_alias_from_tokens(tokens: &[String]) -> String {
    let mut alias = String::new();
    for token in tokens {
        if alias.len() >= 5 {
            break;
        }
        if let Some(ch) = token.chars().find(|c| c.is_ascii_alphabetic()) {
            alias.push(ch.to_ascii_lowercase());
        }
    }
    if alias.is_empty() {
        for token in tokens {
            for ch in token.chars() {
                if ch.is_ascii_alphanumeric() {
                    alias.push(ch.to_ascii_lowercase());
                    if alias.len() >= 5 {
                        break;
                    }
                }
            }
            if alias.len() >= 5 {
                break;
            }
        }
    }
    if alias.is_empty() {
        alias.push_str("grp");
    }
    if alias.len() > 5 {
        alias.truncate(5);
    }
    alias
}

#[inline]
fn fast_split_whitespace_insert(s: &str, out: &mut AHashSet<String>) {
    for cls in s.split_whitespace() {
        if !cls.is_empty() {
            out.insert(cls.to_owned());
        }
    }
}

#[inline]
fn sanitize_group_token(raw: &str) -> &str {
    if let Some(rest) = raw.strip_prefix('@') {
        if !rest.is_empty() {
            return rest;
        }
    }
    raw
}
#[inline]
fn expand_grouping_into(s: &str, out: &mut AHashSet<String>, collector: &mut GroupCollector) {
    let s = match s.as_bytes().iter().position(|&b| b == b'#') {
        Some(i) => &s[..i],
        None => s,
    };
    if !s.as_bytes().iter().any(|&b| matches!(b, b'(' | b')' | b'+')) {
        fast_split_whitespace_insert(s, out);
        return;
    }

    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0usize;
    let mut stack: SmallVec<[String; 4]> = SmallVec::new();
    let mut tok_start: Option<usize> = None;

    #[inline]
    fn trim_plus(s: &str) -> (&str, bool) {
        let mut end = s.len();
        let b = s.as_bytes();
        let mut had_plus = false;
        while end > 0 && b[end - 1] == b'+' {
            end -= 1;
            had_plus = true;
        }
        (&s[..end], had_plus)
    }

    while i < n {
        while i < n && matches!(bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }

        while i < n && bytes[i] == b')' {
            if let Some(ts) = tok_start.take() {
                if ts < i {
                    let raw = &s[ts..i];
                    let (trimmed, had_plus) = trim_plus(raw);
                    let sanitized = sanitize_group_token(trimmed);
                    if !sanitized.is_empty() {
                        let combined = if stack.is_empty() {
                            sanitized.to_string()
                        } else {
                            let total_len =
                                stack.iter().map(|p| p.len() + 1).sum::<usize>() + sanitized.len();
                            let mut combined = String::with_capacity(total_len);
                            for (idx, p) in stack.iter().enumerate() {
                                if idx > 0 {
                                    combined.push(':');
                                }
                                combined.push_str(p);
                            }
                            combined.push(':');
                            combined.push_str(sanitized);
                            combined
                        };
                        out.insert(combined.clone());
                        collector.record(stack.as_slice(), sanitized, had_plus, &combined);
                    }
                }
            }
            if !stack.is_empty() {
                stack.pop();
            }
            i += 1;
            while i < n && matches!(bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
                i += 1;
            }
        }
        if i >= n {
            break;
        }

        if tok_start.is_none() {
            tok_start = Some(i);
        }

        while i < n && !matches!(bytes[i], b' ' | b'\n' | b'\r' | b'\t' | b'(' | b')') {
            i += 1;
        }

        if i < n && bytes[i] == b'(' {
            if let Some(ts) = tok_start.take() {
                if ts < i {
                    let raw = &s[ts..i];
                    let (trimmed, _) = trim_plus(raw);
                    let sanitized = sanitize_group_token(trimmed);
                    if !sanitized.is_empty() {
                        stack.push(sanitized.to_string());
                    }
                }
            }
            i += 1; // consume '('
            continue;
        }

        if let Some(ts) = tok_start.take() {
            if ts < i {
                let raw = &s[ts..i];
                let (trimmed, had_plus) = trim_plus(raw);
                let sanitized = sanitize_group_token(trimmed);
                if !sanitized.is_empty() {
                    let combined = if stack.is_empty() {
                        sanitized.to_string()
                    } else {
                        let total_len =
                            stack.iter().map(|p| p.len() + 1).sum::<usize>() + sanitized.len();
                        let mut combined = String::with_capacity(total_len);
                        for (idx, p) in stack.iter().enumerate() {
                            if idx > 0 {
                                combined.push(':');
                            }
                            combined.push_str(p);
                        }
                        combined.push(':');
                        combined.push_str(sanitized);
                        combined
                    };
                    out.insert(combined.clone());
                    collector.record(stack.as_slice(), sanitized, had_plus, &combined);
                }
            }
        }
    }

    if let Some(ts) = tok_start.take() {
        if ts < n {
            let raw = &s[ts..n];
            let (trimmed, had_plus) = trim_plus(raw);
            let sanitized = sanitize_group_token(trimmed);
            if !sanitized.is_empty() {
                let combined = if stack.is_empty() {
                    sanitized.to_string()
                } else {
                    let total_len =
                        stack.iter().map(|p| p.len() + 1).sum::<usize>() + sanitized.len();
                    let mut combined = String::with_capacity(total_len);
                    for (idx, p) in stack.iter().enumerate() {
                        if idx > 0 {
                            combined.push(':');
                        }
                        combined.push_str(p);
                    }
                    combined.push(':');
                    combined.push_str(sanitized);
                    combined
                };
                out.insert(combined.clone());
                collector.record(stack.as_slice(), sanitized, had_plus, &combined);
            }
        }
    }
}

pub fn extract_classes_fast(html_bytes: &[u8], capacity_hint: usize) -> ExtractedClasses {
    let mut set = AHashSet::with_capacity(capacity_hint.max(64));
    let mut collector = GroupCollector::default();
    let mut pos = 0usize;
    let n = html_bytes.len();

    let class_finder = Finder::new(b"class");
    while let Some(idx) = class_finder.find(&html_bytes[pos..]) {
        let start = pos + idx + 5;
        let mut i = start;
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }
        if i >= n || html_bytes[i] != b'=' {
            pos = start;
            continue;
        }
        i += 1;
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }
        let quote = html_bytes[i];
        if quote != b'"' && quote != b'\'' {
            pos = i;
            continue;
        }
        i += 1;
        let value_start = i;
        let rel_end = memchr(quote, &html_bytes[value_start..]);
        let value_end = match rel_end {
            Some(off) => value_start + off,
            None => break,
        };
        if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
            expand_grouping_into(value_str, &mut set, &mut collector);
        }
        pos = value_end + 1;
    }

    pos = 0;
    let dx_finder = Finder::new(b"dx-");
    while let Some(idx) = dx_finder.find(&html_bytes[pos..]) {
        let mut i = pos + idx;
        i += 3;
        while i < n {
            let b = html_bytes[i];
            if (b as char).is_ascii_alphanumeric() || b == b'-' || b == b'_' {
                i += 1;
            } else {
                break;
            }
        }
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }
        if i >= n || html_bytes[i] != b'=' {
            pos = pos + idx + 3;
            continue;
        }
        i += 1;
        while i < n && matches!(html_bytes[i], b' ' | b'\n' | b'\r' | b'\t') {
            i += 1;
        }
        if i >= n {
            break;
        }
        let quote = html_bytes[i];
        if quote != b'"' && quote != b'\'' {
            pos = pos + idx + 3;
            continue;
        }
        i += 1;
        let value_start = i;
        let rel_end = memchr(quote, &html_bytes[value_start..]);
        let value_end = match rel_end {
            Some(off) => value_start + off,
            None => break,
        };
        if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
            expand_grouping_into(value_str, &mut set, &mut collector);
        }
        pos = value_end + 1;
    }

    ExtractedClasses {
        classes: set,
        group_events: collector.into_events(),
    }
}

#[derive(Debug)]
struct ClassOccurrence {
    attr_range: Range<usize>,
    value_range: Range<usize>,
    token_ranges: SmallVec<[Range<usize>; 8]>,
    canonical_hash: u64,
    dx_group_cleanup: Option<(Range<usize>, String)>,
}

pub fn rewrite_duplicate_classes(html_bytes: &[u8]) -> Option<AutoGroupRewrite> {
    fn is_space(byte: u8) -> bool {
        matches!(byte, b' ' | b'\n' | b'\r' | b'\t')
    }

    let mut occurrences: Vec<ClassOccurrence> = Vec::new();
    let mut existing_names: AHashSet<String> = AHashSet::default();

    let mut pos = 0usize;
    let n = html_bytes.len();
    let finder = Finder::new(b"class");
    while let Some(idx) = finder.find(&html_bytes[pos..]) {
        let attr_start = pos + idx;
        if attr_start > 0 {
            let prev = html_bytes[attr_start - 1];
            if (prev as char).is_ascii_alphanumeric() || prev == b'-' || prev == b'_' {
                pos = attr_start + 5;
                continue;
            }
        }
        let mut cursor = attr_start + 5;
        while cursor < n && is_space(html_bytes[cursor]) {
            cursor += 1;
        }
        if cursor >= n || html_bytes[cursor] != b'=' {
            pos = attr_start + 5;
            continue;
        }
        cursor += 1;
        while cursor < n && is_space(html_bytes[cursor]) {
            cursor += 1;
        }
        if cursor >= n {
            break;
        }
        let quote = html_bytes[cursor];
        if quote != b'"' && quote != b'\'' {
            pos = attr_start + 5;
            continue;
        }
        cursor += 1;
        let value_start = cursor;
        let rel_end = memchr(quote, &html_bytes[value_start..]);
        let value_end = match rel_end {
            Some(off) => value_start + off,
            None => break,
        };
        let attr_end = value_end + 1;
        if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
            let value_bytes = value_str.as_bytes();
            let mut token_ranges: SmallVec<[Range<usize>; 8]> = SmallVec::new();
            let mut cursor_local = 0usize;
            let mut has_disqualifying_char = false;
            while cursor_local < value_bytes.len() {
                while cursor_local < value_bytes.len() && is_space(value_bytes[cursor_local]) {
                    cursor_local += 1;
                }
                if cursor_local >= value_bytes.len() {
                    break;
                }
                let token_start = cursor_local;
                while cursor_local < value_bytes.len() && !is_space(value_bytes[cursor_local]) {
                    let b = value_bytes[cursor_local];
                    if matches!(b, b'(' | b')' | b'{' | b'}' | b':' | b'@' | b'#' | b'[' | b']') {
                        has_disqualifying_char = true;
                    }
                    cursor_local += 1;
                }
                let token_end = cursor_local;
                if token_end > token_start {
                    token_ranges.push(token_start..token_end);
                }
            }

            for range in &token_ranges {
                let token = &value_str[range.clone()];
                if !existing_names.contains(token) {
                    existing_names.insert(token.to_string());
                }
            }

            if token_ranges.len() < 2 {
                pos = attr_end;
                continue;
            }

            let mut canonical_hasher = AHasher::default();
            let mut simple = !has_disqualifying_char;
            for (idx_token, range) in token_ranges.iter().enumerate() {
                let token = &value_str[range.clone()];
                let token_bytes = token.as_bytes();
                if token_bytes.contains(&b'+') || token_bytes.starts_with(b"dxg-") {
                    simple = false;
                }
                for prev_range in &token_ranges[..idx_token] {
                    let prev_token = &value_bytes[prev_range.clone()];
                    if prev_token.len() == token_bytes.len() && prev_token == token_bytes {
                        simple = false;
                        break;
                    }
                }
                canonical_hasher.write(token_bytes);
                canonical_hasher.write_u8(0);
                if !simple {
                    break;
                }
            }

            if !simple {
                pos = attr_end;
                continue;
            }

            let whitespace_start = attr_end;
            let mut attr_ws_end = whitespace_start;
            while attr_ws_end < n && is_space(html_bytes[attr_ws_end]) {
                attr_ws_end += 1;
            }
            let mut dx_group_cleanup: Option<(Range<usize>, String)> = None;
            if attr_ws_end + 8 <= n && &html_bytes[attr_ws_end..attr_ws_end + 8] == b"dx-group" {
                let mut cursor_after_name = attr_ws_end + 8;
                while cursor_after_name < n && is_space(html_bytes[cursor_after_name]) {
                    cursor_after_name += 1;
                }
                if cursor_after_name < n && html_bytes[cursor_after_name] == b'=' {
                    cursor_after_name += 1;
                    while cursor_after_name < n && is_space(html_bytes[cursor_after_name]) {
                        cursor_after_name += 1;
                    }
                    if cursor_after_name < n {
                        let dx_quote = html_bytes[cursor_after_name];
                        if dx_quote == b'"' || dx_quote == b'\'' {
                            cursor_after_name += 1;
                            let value_start = cursor_after_name;
                            if let Some(off) = memchr(dx_quote, &html_bytes[value_start..]) {
                                let value_end = value_start + off;
                                cursor_after_name = value_end + 1;
                                let mut trailing = cursor_after_name;
                                while trailing < n && is_space(html_bytes[trailing]) {
                                    trailing += 1;
                                }
                                let has_following_attr =
                                    trailing < n && !matches!(html_bytes[trailing], b'>' | b'/');
                                let prefix_bytes = &html_bytes[whitespace_start..attr_ws_end];
                                let prefix =
                                    std::str::from_utf8(prefix_bytes).unwrap_or(" ").to_string();
                                let replacement = if has_following_attr {
                                    if prefix.is_empty() {
                                        " ".to_string()
                                    } else {
                                        prefix
                                    }
                                } else {
                                    String::new()
                                };
                                dx_group_cleanup = Some((whitespace_start..trailing, replacement));
                            }
                        }
                    }
                }
            }

            occurrences.push(ClassOccurrence {
                attr_range: attr_start..attr_end,
                value_range: value_start..value_end,
                token_ranges,
                canonical_hash: canonical_hasher.finish(),
                dx_group_cleanup,
            });
        }
        pos = attr_end;
    }

    {
        let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
        let mut pos_scan = 0usize;
        let n_all = html_bytes.len();
        let finder = Finder::new(b"class");
        while let Some(idx) = finder.find(&html_bytes[pos_scan..]) {
            let attr_start = pos_scan + idx;
            if attr_start > 0 {
                let prev = html_bytes[attr_start - 1];
                if (prev as char).is_ascii_alphanumeric() || prev == b'-' || prev == b'_' {
                    pos_scan = attr_start + 5;
                    continue;
                }
            }
            let mut cursor = attr_start + 5;
            while cursor < n_all && is_space(html_bytes[cursor]) {
                cursor += 1;
            }
            if cursor >= n_all || html_bytes[cursor] != b'=' {
                pos_scan = attr_start + 5;
                continue;
            }
            cursor += 1;
            while cursor < n_all && is_space(html_bytes[cursor]) {
                cursor += 1;
            }
            if cursor >= n_all {
                break;
            }
            let quote = html_bytes[cursor];
            if quote != b'"' && quote != b'\'' {
                pos_scan = attr_start + 5;
                continue;
            }
            cursor += 1;
            let value_start = cursor;
            if let Some(rel_end) = memchr(quote, &html_bytes[value_start..]) {
                let value_end = value_start + rel_end;
                if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
                    let mut out_tokens: Vec<String> = Vec::new();
                    let mut changed = false;
                    let tokens: Vec<&str> = value_str.split_whitespace().collect();
                    let mut ti = 0usize;
                    while ti < tokens.len() {
                        let tk = tokens[ti];
                        if tk.starts_with('@') {
                            out_tokens.push(tk.to_string());
                            ti += 1;
                            continue;
                        }
                        if let Some(open_idx) = tk.find('(') {
                            let name = &tk[..open_idx];
                            let mut found_close = tk.ends_with(')');
                            let mut j = ti + 1;
                            while j < tokens.len() && !found_close {
                                if tokens[j].ends_with(')') {
                                    found_close = true;
                                    j += 1;
                                    break;
                                }
                                j += 1;
                            }
                            if !name.is_empty() {
                                out_tokens.push(name.to_string());
                            }
                            if found_close {
                                changed = true;
                                ti = j;
                                continue;
                            } else {
                                out_tokens.push(tk.to_string());
                                ti += 1;
                                continue;
                            }
                        }
                        out_tokens.push(tk.to_string());
                        ti += 1;
                    }
                    if changed {
                        let replacement = if out_tokens.is_empty() {
                            String::new()
                        } else {
                            format!("class=\"{}\"", out_tokens.join(" "))
                        };
                        replacements.push((attr_start..(value_end + 1), replacement));
                    }
                }
                pos_scan = value_end + 1;
                continue;
            } else {
                break;
            }
        }
        if !replacements.is_empty() {
            replacements.sort_by(|a, b| b.0.start.cmp(&a.0.start));
            let mut html_string = String::from_utf8(html_bytes.to_vec()).ok()?;
            for (range, replacement) in replacements {
                html_string.replace_range(range, &replacement);
            }
            return Some(AutoGroupRewrite {
                html: html_string.into_bytes(),
                groups: Vec::new(),
            });
        }
    }

    if occurrences.is_empty() {
        let mut manual_aliases: AHashMap<String, Vec<String>> = AHashMap::default();
        let mut pos_scan = 0usize;
        let n_all = html_bytes.len();
        let finder = Finder::new(b"class");
        while let Some(idx) = finder.find(&html_bytes[pos_scan..]) {
            let attr_start = pos_scan + idx;
            if attr_start > 0 {
                let prev = html_bytes[attr_start - 1];
                if (prev as char).is_ascii_alphanumeric() || prev == b'-' || prev == b'_' {
                    pos_scan = attr_start + 5;
                    continue;
                }
            }
            let mut cursor = attr_start + 5;
            while cursor < n_all && is_space(html_bytes[cursor]) {
                cursor += 1;
            }
            if cursor >= n_all || html_bytes[cursor] != b'=' {
                pos_scan = attr_start + 5;
                continue;
            }
            cursor += 1;
            while cursor < n_all && is_space(html_bytes[cursor]) {
                cursor += 1;
            }
            if cursor >= n_all {
                break;
            }
            let quote = html_bytes[cursor];
            if quote != b'"' && quote != b'\'' {
                pos_scan = attr_start + 5;
                continue;
            }
            cursor += 1;
            let value_start = cursor;
            if let Some(rel_end) = memchr(quote, &html_bytes[value_start..]) {
                let value_end = value_start + rel_end;
                if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
                    let tokens: Vec<&str> = value_str.split_whitespace().collect();
                    let mut ti = 0usize;
                    while ti < tokens.len() {
                        let tk = tokens[ti];
                        if tk.starts_with('@') {
                            ti += 1;
                            continue;
                        }
                        if let Some(open_idx) = tk.find('(') {
                            let name = &tk[..open_idx];
                            let mut inner_tokens: Vec<String> = Vec::new();
                            let first_rem = &tk[open_idx + 1..];
                            if !first_rem.is_empty() {
                                inner_tokens.push(first_rem.trim_end_matches(')').to_string());
                            }
                            let mut found_close = tk.ends_with(')');
                            let mut j = ti + 1;
                            while j < tokens.len() && !found_close {
                                let tk2 = tokens[j];
                                if tk2.ends_with(')') {
                                    inner_tokens.push(tk2.trim_end_matches(')').to_string());
                                    found_close = true;
                                    j += 1;
                                    break;
                                } else {
                                    inner_tokens.push(tk2.to_string());
                                }
                                j += 1;
                            }
                            if found_close {
                                inner_tokens.retain(|s| !s.is_empty());
                                if !name.is_empty() && !inner_tokens.is_empty() {
                                    manual_aliases.entry(name.to_string()).or_insert(inner_tokens);
                                }
                                ti = j;
                                continue;
                            }
                        }
                        ti += 1;
                    }
                }
                pos_scan = value_end + 1;
                continue;
            } else {
                break;
            }
        }

        if manual_aliases.is_empty() {
            return None;
        }

        let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
        let mut pos2 = 0usize;
        let n = html_bytes.len();
        let finder = Finder::new(b"class");
        while let Some(idx) = finder.find(&html_bytes[pos2..]) {
            let attr_start = pos2 + idx;
            if attr_start > 0 {
                let prev = html_bytes[attr_start - 1];
                if (prev as char).is_ascii_alphanumeric() || prev == b'-' || prev == b'_' {
                    pos2 = attr_start + 5;
                    continue;
                }
            }
            let mut cursor = attr_start + 5;
            while cursor < n && matches!(html_bytes[cursor], b' ' | b'\n' | b'\r' | b'\t') {
                cursor += 1;
            }
            if cursor >= n || html_bytes[cursor] != b'=' {
                pos2 = attr_start + 5;
                continue;
            }
            cursor += 1;
            while cursor < n && matches!(html_bytes[cursor], b' ' | b'\n' | b'\r' | b'\t') {
                cursor += 1;
            }
            if cursor >= n {
                break;
            }
            let quote = html_bytes[cursor];
            if quote != b'"' && quote != b'\'' {
                pos2 = attr_start + 5;
                continue;
            }
            cursor += 1;
            let value_start = cursor;
            if let Some(rel_end) = memchr(quote, &html_bytes[value_start..]) {
                let value_end = value_start + rel_end;
                if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
                    let mut out_tokens: Vec<String> = Vec::new();
                    let mut changed = false;
                    for tok in value_str.split_whitespace() {
                        if tok.starts_with('@') {
                            out_tokens.push(tok.to_string());
                            continue;
                        }
                        let mut skipped = false;
                        if let Some(open_idx) = tok.find('(') {
                            if tok.ends_with(')') {
                                let name = &tok[..open_idx];
                                if manual_aliases.contains_key(name) {
                                    skipped = true;
                                }
                            }
                        }
                        if manual_aliases.contains_key(tok) {
                            skipped = true;
                        }
                        if skipped {
                            changed = true;
                            continue;
                        }
                        out_tokens.push(tok.to_string());
                    }
                    if changed {
                        let replacement = if out_tokens.is_empty() {
                            String::new()
                        } else {
                            format!("class=\"{}\"", out_tokens.join(" "))
                        };
                        replacements.push((attr_start..(value_end + 1), replacement));
                    }
                }
                pos2 = value_end + 1;
                continue;
            } else {
                break;
            }
        }

        let mut pos2 = 0usize;
        let n = html_bytes.len();
        let finder = Finder::new(b"class");
        while let Some(idx) = finder.find(&html_bytes[pos2..]) {
            let attr_start = pos2 + idx;
            if attr_start > 0 {
                let prev = html_bytes[attr_start - 1];
                if (prev as char).is_ascii_alphanumeric() || prev == b'-' || prev == b'_' {
                    pos2 = attr_start + 5;
                    continue;
                }
            }
            let mut cursor = attr_start + 5;
            while cursor < n && matches!(html_bytes[cursor], b' ' | b'\n' | b'\r' | b'\t') {
                cursor += 1;
            }
            if cursor >= n || html_bytes[cursor] != b'=' {
                pos2 = attr_start + 5;
                continue;
            }
            cursor += 1;
            while cursor < n && matches!(html_bytes[cursor], b' ' | b'\n' | b'\r' | b'\t') {
                cursor += 1;
            }
            if cursor >= n {
                break;
            }
            let quote = html_bytes[cursor];
            if quote != b'"' && quote != b'\'' {
                pos2 = attr_start + 5;
                continue;
            }
            cursor += 1;
            let value_start = cursor;
            if let Some(rel_end) = memchr(quote, &html_bytes[value_start..]) {
                let value_end = value_start + rel_end;
                if let Ok(value_str) = std::str::from_utf8(&html_bytes[value_start..value_end]) {
                    if !value_str.contains(char::is_whitespace) {
                        if manual_aliases.contains_key(value_str) {
                            replacements.push((attr_start..(value_end + 2), String::new()));
                        } else if let Some(open_idx) = value_str.find('(') {
                            if value_str.ends_with(')') {
                                let name = &value_str[..open_idx];
                                if manual_aliases.contains_key(name) {
                                    replacements.push((attr_start..(value_end + 2), String::new()));
                                }
                            }
                        }
                    }
                }
                pos2 = value_end + 1;
                continue;
            } else {
                break;
            }
        }

        if replacements.is_empty() {
            return None;
        }
        replacements.sort_by(|a, b| b.0.start.cmp(&a.0.start));
        let mut html_string = String::from_utf8(html_bytes.to_vec()).ok()?;
        for (range, replacement) in replacements {
            html_string.replace_range(range, &replacement);
        }
        return Some(AutoGroupRewrite {
            html: html_string.into_bytes(),
            groups: Vec::new(),
        });
    }

    let mut grouped: AHashMap<u64, Vec<usize>> = AHashMap::default();
    for (idx, occ) in occurrences.iter().enumerate() {
        grouped.entry(occ.canonical_hash).or_default().push(idx);
    }

    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
    let mut infos: Vec<AutoGroupInfo> = Vec::new();
    let mut alias_counts: AHashMap<String, usize> = AHashMap::default();

    for indices in grouped.values() {
        if indices.len() < 2 {
            continue;
        }
        let mut remaining = indices.clone();
        while !remaining.is_empty() {
            let current_idx = remaining.swap_remove(0);
            let mut matches = vec![current_idx];
            let mut j = 0;
            while j < remaining.len() {
                let candidate_idx = remaining[j];
                if classes_match(&occurrences[current_idx], &occurrences[candidate_idx], html_bytes)
                {
                    matches.push(candidate_idx);
                    remaining.swap_remove(j);
                } else {
                    j += 1;
                }
            }

            if matches.len() < 2 {
                continue;
            }

            matches.sort_by_key(|&idx| occurrences[idx].attr_range.start);
            let first_idx = matches[0];
            let first_occ = &occurrences[first_idx];
            let value_slice = &html_bytes[first_occ.value_range.clone()];
            let value_str = std::str::from_utf8(value_slice).ok()?;
            let mut tokens: Vec<String> = Vec::with_capacity(first_occ.token_ranges.len());
            for range in &first_occ.token_ranges {
                tokens.push(value_str[range.clone()].to_string());
            }

            let base_alias = base_alias_from_tokens(&tokens);
            let mut suffix = alias_counts.get(&base_alias).copied().unwrap_or(0);
            let mut candidate = if suffix == 0 {
                base_alias.clone()
            } else {
                format!("{}{}", base_alias, suffix)
            };
            while existing_names.contains(candidate.as_str()) {
                suffix += 1;
                candidate = format!("{}{}", base_alias, suffix);
            }
            alias_counts.insert(base_alias.clone(), suffix + 1);
            existing_names.insert(candidate.clone());
            let alias = candidate;
            let tokens_join = tokens.join(" ");
            let first_range = first_occ.attr_range.clone();
            replacements.push((first_range, format!("class=\"@{}({})\"", alias, tokens_join)));
            if let Some((range, replacement)) = first_occ.dx_group_cleanup.clone() {
                replacements.push((range, replacement));
            }
            for &idx in &matches[1..] {
                let occ = &occurrences[idx];
                replacements.push((occ.attr_range.clone(), format!("class=\"{}\"", alias)));
                if let Some((range, replacement)) = occ.dx_group_cleanup.clone() {
                    replacements.push((range, replacement));
                }
            }
            infos.push(AutoGroupInfo {
                alias,
                classes: tokens,
            });
        }
    }

    if replacements.is_empty() {
        return None;
    }

    replacements.sort_by(|a, b| b.0.start.cmp(&a.0.start));
    let mut html_string = String::from_utf8(html_bytes.to_vec()).ok()?;
    for (range, replacement) in replacements {
        html_string.replace_range(range, &replacement);
    }

    Some(AutoGroupRewrite {
        html: html_string.into_bytes(),
        groups: infos,
    })
}

fn classes_match(a: &ClassOccurrence, b: &ClassOccurrence, html_bytes: &[u8]) -> bool {
    if a.token_ranges.len() != b.token_ranges.len() {
        return false;
    }
    let a_slice = &html_bytes[a.value_range.clone()];
    let b_slice = &html_bytes[b.value_range.clone()];
    for (range_a, range_b) in a.token_ranges.iter().zip(&b.token_ranges) {
        if range_a.len() != range_b.len() {
            return false;
        }
        if a_slice[range_a.clone()] != b_slice[range_b.clone()] {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouping_extracts_alias_and_utilities() {
        let html = br#"<div dx-text="card(bg-red-500 h-50 text-yellow-500+)"></div>"#;
        let extracted = extract_classes_fast(html, 0);
        assert!(
            !extracted.classes.contains("bg-red-500"),
            "base utility should be restored during analysis"
        );
        assert!(
            !extracted.classes.contains("h-50"),
            "base utility should be restored during analysis"
        );
        assert!(
            !extracted.classes.contains("text-yellow-500"),
            "base utility should be restored during analysis"
        );
        assert!(
            !extracted.classes.contains("card"),
            "alias should be added during analysis, not raw extraction"
        );
        assert!(extracted.classes.contains("card:bg-red-500"));
        let alias_event = extracted
            .group_events
            .iter()
            .find(|evt| evt.stack == vec!["card".to_string()] && evt.token == "text-yellow-500");
        assert!(alias_event.is_some(), "expected event for card group");
        assert!(alias_event.unwrap().had_plus, "plus suffix should be recorded");

        let mut classes = extracted.classes.clone();
        let registry =
            crate::core::group::GroupRegistry::analyze(&extracted.group_events, &mut classes, None);
        assert!(classes.contains("card"), "alias should be registered after analysis");
        assert!(classes.contains("bg-red-500"));
        assert!(classes.contains("h-50"));
        assert!(classes.contains("text-yellow-500"));
        assert!(
            !classes.contains("card:bg-red-500"),
            "internal grouped tokens should be dropped after analysis"
        );
        assert!(
            registry.definitions().any(|(name, _)| name == "card"),
            "registry should capture alias definition"
        );
    }

    #[test]
    fn rewrite_duplicates_into_group_alias() {
        let html = br#"<h1 class="border flex text-red-500" dx-group="old(alias)">Hello</h1>
<h1 class="border flex text-red-500">World</h1>"#;
        let result = rewrite_duplicate_classes(html).expect("rewrite");
        let rewritten = String::from_utf8(result.html.clone()).unwrap();
        assert!(
            rewritten.contains("class=\"@bft(border flex text-red-500)\""),
            "expected alias definition to use grouped class without duplicate alias"
        );
        assert!(
            rewritten.contains("class=\"bft\">World"),
            "expected second occurrence to use alias only"
        );
        assert!(!rewritten.contains("dx-group"), "legacy dx-group attribute should be stripped");
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].alias, "bft");
        assert_eq!(
            result.groups[0].classes,
            vec![
                "border".to_string(),
                "flex".to_string(),
                "text-red-500".to_string()
            ]
        );
    }

    #[test]
    fn rewrite_alias_avoids_existing_names() {
        let html = br#"<div class="bft">Existing</div>
<div class="border flex text-red-500">Hello</div>
<div class="border flex text-red-500">World</div>"#;
        let result = rewrite_duplicate_classes(html).expect("rewrite");
        let rewritten = String::from_utf8(result.html.clone()).unwrap();
        assert!(rewritten.contains("class=\"bft\">Existing"), "untouched class should remain");
        assert!(
            rewritten.contains("class=\"@bft1(border flex text-red-500)\""),
            "expected alias to avoid collision via numeric suffix"
        );
        assert!(
            rewritten.contains("class=\"bft1\">World"),
            "expected subsequent match to use the alias"
        );
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].alias, "bft1");
    }

    #[test]
    fn rewrite_alias_truncates_to_five_letters() {
        let html = br#"<div class="border flex uppercase text-red-500 after shadow">One</div>
<div class="border flex uppercase text-red-500 after shadow">Two</div>"#;
        let result = rewrite_duplicate_classes(html).expect("rewrite");
        let rewritten = String::from_utf8(result.html.clone()).unwrap();
        assert!(
            rewritten.contains("class=\"@bfuta(border flex uppercase text-red-500 after shadow)\""),
            "expected alias to use first five initials"
        );
        assert!(
            result.groups.iter().any(|g| g.alias == "bfuta"),
            "alias metadata should record truncated name"
        );
    }
}
