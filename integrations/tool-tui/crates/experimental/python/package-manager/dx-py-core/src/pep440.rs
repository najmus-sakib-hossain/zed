//! PEP 440 Version Parsing and Comparison
//!
//! Full implementation of PEP 440 version identification and comparison.
//! Supports epoch, release segments, pre/post/dev releases, and local versions.
//!
//! # Examples
//! ```
//! use dx_py_core::pep440::Pep440Version;
//!
//! let v1 = Pep440Version::parse("1.0.0a1").unwrap();
//! let v2 = Pep440Version::parse("1.0.0").unwrap();
//! assert!(v1 < v2); // pre-release < release
//!
//! let v3 = Pep440Version::parse("1!0.0.0").unwrap();
//! assert!(v3 > v2); // epoch 1 > epoch 0
//! ```

use std::cmp::Ordering;
use std::fmt;
use std::str::FromStr;

/// Pre-release type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PreRelease {
    /// Alpha release (a, alpha)
    Alpha(u32),
    /// Beta release (b, beta)
    Beta(u32),
    /// Release candidate (rc, c, pre, preview)
    ReleaseCandidate(u32),
}

impl PreRelease {
    /// Get ordering priority (lower = earlier in release cycle)
    fn priority(&self) -> u8 {
        match self {
            PreRelease::Alpha(_) => 0,
            PreRelease::Beta(_) => 1,
            PreRelease::ReleaseCandidate(_) => 2,
        }
    }

    /// Get the numeric part
    fn number(&self) -> u32 {
        match self {
            PreRelease::Alpha(n) | PreRelease::Beta(n) | PreRelease::ReleaseCandidate(n) => *n,
        }
    }
}

impl PartialOrd for PreRelease {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PreRelease {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority().cmp(&other.priority()) {
            Ordering::Equal => self.number().cmp(&other.number()),
            ord => ord,
        }
    }
}

impl fmt::Display for PreRelease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreRelease::Alpha(n) => write!(f, "a{}", n),
            PreRelease::Beta(n) => write!(f, "b{}", n),
            PreRelease::ReleaseCandidate(n) => write!(f, "rc{}", n),
        }
    }
}

/// Full PEP 440 version representation
///
/// Supports all PEP 440 version components:
/// - Epoch (N!)
/// - Release segments (N.N.N...)
/// - Pre-release (aN, bN, rcN)
/// - Post-release (.postN)
/// - Dev release (.devN)
/// - Local version (+local)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pep440Version {
    /// Epoch (default 0)
    pub epoch: u32,
    /// Release segments (e.g., [1, 2, 3] for 1.2.3)
    pub release: Vec<u32>,
    /// Pre-release (alpha, beta, rc)
    pub pre: Option<PreRelease>,
    /// Post-release number
    pub post: Option<u32>,
    /// Dev release number
    pub dev: Option<u32>,
    /// Local version identifier
    pub local: Option<String>,
}

impl Default for Pep440Version {
    fn default() -> Self {
        Self {
            epoch: 0,
            release: vec![0],
            pre: None,
            post: None,
            dev: None,
            local: None,
        }
    }
}

impl Pep440Version {
    /// Create a new version with just release segments
    pub fn new(release: Vec<u32>) -> Self {
        Self {
            epoch: 0,
            release,
            pre: None,
            post: None,
            dev: None,
            local: None,
        }
    }

    /// Create a simple X.Y.Z version
    pub fn simple(major: u32, minor: u32, patch: u32) -> Self {
        Self::new(vec![major, minor, patch])
    }

    /// Parse a PEP 440 version string
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        s.parse()
    }

    /// Check if this is a pre-release version
    pub fn is_prerelease(&self) -> bool {
        self.pre.is_some() || self.dev.is_some()
    }

    /// Check if this is a post-release version
    pub fn is_postrelease(&self) -> bool {
        self.post.is_some()
    }

    /// Check if this is a dev release
    pub fn is_devrelease(&self) -> bool {
        self.dev.is_some()
    }

    /// Check if this has a local version
    pub fn is_local(&self) -> bool {
        self.local.is_some()
    }

    /// Get the base version (without pre/post/dev/local)
    pub fn base_version(&self) -> Self {
        Self {
            epoch: self.epoch,
            release: self.release.clone(),
            pre: None,
            post: None,
            dev: None,
            local: None,
        }
    }

    /// Get release segment at index, defaulting to 0
    fn release_segment(&self, idx: usize) -> u32 {
        self.release.get(idx).copied().unwrap_or(0)
    }
}

/// Parse error for PEP 440 versions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Empty version string
    Empty,
    /// Invalid epoch format
    InvalidEpoch(String),
    /// Invalid release segment
    InvalidRelease(String),
    /// Invalid pre-release format
    InvalidPreRelease(String),
    /// Invalid post-release format
    InvalidPostRelease(String),
    /// Invalid dev release format
    InvalidDevRelease(String),
    /// Invalid local version format
    InvalidLocal(String),
    /// General parse error
    Invalid(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty version string"),
            ParseError::InvalidEpoch(s) => write!(f, "invalid epoch: {}", s),
            ParseError::InvalidRelease(s) => write!(f, "invalid release: {}", s),
            ParseError::InvalidPreRelease(s) => write!(f, "invalid pre-release: {}", s),
            ParseError::InvalidPostRelease(s) => write!(f, "invalid post-release: {}", s),
            ParseError::InvalidDevRelease(s) => write!(f, "invalid dev release: {}", s),
            ParseError::InvalidLocal(s) => write!(f, "invalid local version: {}", s),
            ParseError::Invalid(s) => write!(f, "invalid version: {}", s),
        }
    }
}

impl std::error::Error for ParseError {}

impl FromStr for Pep440Version {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(ParseError::Empty);
        }

        // Normalize: lowercase, handle v prefix
        let s = s.to_lowercase();
        let s = s.strip_prefix('v').unwrap_or(&s);

        let mut version = Pep440Version::default();
        let mut remaining = s;

        // Parse epoch (N!)
        if let Some(idx) = remaining.find('!') {
            let epoch_str = &remaining[..idx];
            version.epoch =
                epoch_str.parse().map_err(|_| ParseError::InvalidEpoch(epoch_str.to_string()))?;
            remaining = &remaining[idx + 1..];
        }

        // Parse local version (+local)
        let local_part;
        if let Some(idx) = remaining.find('+') {
            local_part = Some(&remaining[idx + 1..]);
            remaining = &remaining[..idx];
        } else {
            local_part = None;
        }

        // Parse release and modifiers
        let (release_str, modifiers) = split_release_modifiers(remaining);

        // Parse release segments
        if release_str.is_empty() {
            return Err(ParseError::InvalidRelease("empty release".to_string()));
        }

        version.release = parse_release_segments(release_str)?;

        // Parse modifiers (pre, post, dev)
        parse_modifiers(&mut version, modifiers)?;

        // Set local version
        if let Some(local) = local_part {
            if local.is_empty() {
                return Err(ParseError::InvalidLocal("empty local version".to_string()));
            }
            version.local = Some(normalize_local(local));
        }

        Ok(version)
    }
}

/// Split release segments from modifiers
fn split_release_modifiers(s: &str) -> (&str, &str) {
    // Find where release ends and modifiers begin
    // Release is digits and dots, modifiers start with letter or hyphen/underscore
    let mut end = s.len();
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_alphabetic() || c == '-' || c == '_' {
            // Check if this is part of release (shouldn't be)
            end = i;
            break;
        }
        if c == '.' {
            // Check if next char is a digit (still release) or letter (modifier)
            if let Some(&next) = chars.get(i + 1) {
                if next.is_ascii_alphabetic() {
                    end = i;
                    break;
                }
            }
        }
    }

    (&s[..end], &s[end..])
}

/// Parse release segments (e.g., "1.2.3" -> [1, 2, 3])
fn parse_release_segments(s: &str) -> Result<Vec<u32>, ParseError> {
    let s = s.trim_matches('.');
    if s.is_empty() {
        return Err(ParseError::InvalidRelease("empty release".to_string()));
    }

    let mut segments = Vec::new();
    for part in s.split('.') {
        if part.is_empty() {
            continue;
        }
        let num: u32 = part.parse().map_err(|_| ParseError::InvalidRelease(part.to_string()))?;
        segments.push(num);
    }

    if segments.is_empty() {
        return Err(ParseError::InvalidRelease(s.to_string()));
    }

    Ok(segments)
}

/// Parse modifiers (pre, post, dev)
fn parse_modifiers(version: &mut Pep440Version, s: &str) -> Result<(), ParseError> {
    if s.is_empty() {
        return Ok(());
    }

    // Normalize separators
    let normalized = s.replace(['-', '_'], ".");
    let normalized = normalized.trim_start_matches('.');

    let mut remaining: &str = normalized;

    while !remaining.is_empty() {
        remaining = remaining.trim_start_matches('.');

        if remaining.is_empty() {
            break;
        }

        // Try to parse pre-release
        if let Some(rest) = try_parse_pre(remaining, version) {
            remaining = rest;
            continue;
        }

        // Try to parse post-release
        if let Some(rest) = try_parse_post(remaining, version) {
            remaining = rest;
            continue;
        }

        // Try to parse dev release
        if let Some(rest) = try_parse_dev(remaining, version) {
            remaining = rest;
            continue;
        }

        // Unknown modifier
        return Err(ParseError::Invalid(format!("unknown modifier: {}", remaining)));
    }

    Ok(())
}

/// Try to parse pre-release, returns remaining string if successful
fn try_parse_pre<'a>(s: &'a str, version: &mut Pep440Version) -> Option<&'a str> {
    // Check each prefix manually to avoid type issues with function pointers
    if let Some(rest) = s.strip_prefix("alpha") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::Alpha(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("a") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::Alpha(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("beta") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::Beta(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("b") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::Beta(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("preview") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::ReleaseCandidate(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("pre") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::ReleaseCandidate(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("rc") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::ReleaseCandidate(num));
        return Some(remaining);
    }
    if let Some(rest) = s.strip_prefix("c") {
        let (num, remaining) = parse_number_prefix(rest);
        version.pre = Some(PreRelease::ReleaseCandidate(num));
        return Some(remaining);
    }

    None
}

/// Try to parse post-release, returns remaining string if successful
fn try_parse_post<'a>(s: &'a str, version: &mut Pep440Version) -> Option<&'a str> {
    let prefixes = ["post", "rev", "r"];

    for prefix in prefixes {
        if let Some(rest) = s.strip_prefix(prefix) {
            let (num, remaining) = parse_number_prefix(rest);
            version.post = Some(num);
            return Some(remaining);
        }
    }

    // Implicit post release: just a number after release
    if s.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        // Check if this looks like an implicit post (e.g., "-1" normalized to ".1")
        let (num, remaining) = parse_number_prefix(s);
        if version.pre.is_none() && version.post.is_none() {
            version.post = Some(num);
            return Some(remaining);
        }
    }

    None
}

/// Try to parse dev release, returns remaining string if successful
fn try_parse_dev<'a>(s: &'a str, version: &mut Pep440Version) -> Option<&'a str> {
    if let Some(rest) = s.strip_prefix("dev") {
        let (num, remaining) = parse_number_prefix(rest);
        version.dev = Some(num);
        return Some(remaining);
    }
    None
}

/// Parse a number from the start of a string, returns (number, remaining)
fn parse_number_prefix(s: &str) -> (u32, &str) {
    let s = s.trim_start_matches('.');
    let end = s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, _)| i)
        .unwrap_or(s.len());

    if end == 0 {
        (0, s)
    } else {
        let num = s[..end].parse().unwrap_or(0);
        (num, &s[end..])
    }
}

/// Normalize local version identifier
fn normalize_local(s: &str) -> String {
    // Replace separators with dots, lowercase
    s.replace(['-', '_'], ".").to_lowercase()
}

impl fmt::Display for Pep440Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Epoch
        if self.epoch != 0 {
            write!(f, "{}!", self.epoch)?;
        }

        // Release
        let release_str: Vec<String> = self.release.iter().map(|n| n.to_string()).collect();
        write!(f, "{}", release_str.join("."))?;

        // Pre-release
        if let Some(ref pre) = self.pre {
            write!(f, "{}", pre)?;
        }

        // Post-release
        if let Some(post) = self.post {
            write!(f, ".post{}", post)?;
        }

        // Dev release
        if let Some(dev) = self.dev {
            write!(f, ".dev{}", dev)?;
        }

        // Local
        if let Some(ref local) = self.local {
            write!(f, "+{}", local)?;
        }

        Ok(())
    }
}

impl PartialOrd for Pep440Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Pep440Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // 1. Compare epoch
        match self.epoch.cmp(&other.epoch) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // 2. Compare release segments
        let max_len = self.release.len().max(other.release.len());
        for i in 0..max_len {
            match self.release_segment(i).cmp(&other.release_segment(i)) {
                Ordering::Equal => {}
                ord => return ord,
            }
        }

        // 3. Compare pre-release phase
        // Ordering: dev-only < pre+dev < pre < release < post
        // We use a numeric "phase" to simplify comparison
        let self_phase = version_phase(&self.dev, &self.pre);
        let other_phase = version_phase(&other.dev, &other.pre);

        match self_phase.cmp(&other_phase) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // 4. Within same phase, compare details
        // Compare pre-release type and number
        match (&self.pre, &other.pre) {
            (Some(p1), Some(p2)) => match p1.cmp(p2) {
                Ordering::Equal => {}
                ord => return ord,
            },
            (None, None) => {}
            // Different phases would have been caught above
            _ => {}
        }

        // Compare dev numbers within same pre-release
        match (&self.dev, &other.dev) {
            (Some(d1), Some(d2)) => match d1.cmp(d2) {
                Ordering::Equal => {}
                ord => return ord,
            },
            (None, None) => {}
            // dev vs no-dev within same pre would have different phases
            _ => {}
        }

        // 5. Compare post-release
        match (&self.post, &other.post) {
            (Some(p1), Some(p2)) => match p1.cmp(p2) {
                Ordering::Equal => {}
                ord => return ord,
            },
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => {}
        }

        // 6. Compare local version (lexicographic, segments can be numeric or string)
        match (&self.local, &other.local) {
            (Some(l1), Some(l2)) => compare_local(l1, l2),
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
    }
}

/// Calculate version phase for ordering
/// Lower phase = earlier in release cycle
fn version_phase(dev: &Option<u32>, pre: &Option<PreRelease>) -> u8 {
    match (dev, pre) {
        (Some(_), None) => 0,    // dev only (e.g., 1.0.0.dev1)
        (Some(_), Some(_)) => 1, // pre + dev (e.g., 1.0.0a1.dev1)
        (None, Some(_)) => 2,    // pre only (e.g., 1.0.0a1)
        (None, None) => 3,       // release or post
    }
}

/// Compare local version identifiers
fn compare_local(a: &str, b: &str) -> Ordering {
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();

    for (ap, bp) in a_parts.iter().zip(b_parts.iter()) {
        let ord = match (ap.parse::<u32>(), bp.parse::<u32>()) {
            (Ok(an), Ok(bn)) => an.cmp(&bn),
            (Ok(_), Err(_)) => Ordering::Greater, // numeric > string
            (Err(_), Ok(_)) => Ordering::Less,
            (Err(_), Err(_)) => ap.cmp(bp),
        };
        if ord != Ordering::Equal {
            return ord;
        }
    }

    a_parts.len().cmp(&b_parts.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let v = Pep440Version::parse("1.2.3").unwrap();
        assert_eq!(v.release, vec![1, 2, 3]);
        assert_eq!(v.epoch, 0);
        assert!(v.pre.is_none());
        assert!(v.post.is_none());
        assert!(v.dev.is_none());
        assert!(v.local.is_none());
    }

    #[test]
    fn test_parse_epoch() {
        let v = Pep440Version::parse("1!2.0.0").unwrap();
        assert_eq!(v.epoch, 1);
        assert_eq!(v.release, vec![2, 0, 0]);
    }

    #[test]
    fn test_parse_prerelease() {
        let v = Pep440Version::parse("1.0.0a1").unwrap();
        assert_eq!(v.pre, Some(PreRelease::Alpha(1)));

        let v = Pep440Version::parse("1.0.0b2").unwrap();
        assert_eq!(v.pre, Some(PreRelease::Beta(2)));

        let v = Pep440Version::parse("1.0.0rc3").unwrap();
        assert_eq!(v.pre, Some(PreRelease::ReleaseCandidate(3)));

        // Alternative spellings
        let v = Pep440Version::parse("1.0.0alpha1").unwrap();
        assert_eq!(v.pre, Some(PreRelease::Alpha(1)));

        let v = Pep440Version::parse("1.0.0beta2").unwrap();
        assert_eq!(v.pre, Some(PreRelease::Beta(2)));

        let v = Pep440Version::parse("1.0.0c3").unwrap();
        assert_eq!(v.pre, Some(PreRelease::ReleaseCandidate(3)));
    }

    #[test]
    fn test_parse_postrelease() {
        let v = Pep440Version::parse("1.0.0.post1").unwrap();
        assert_eq!(v.post, Some(1));

        let v = Pep440Version::parse("1.0.0-1").unwrap();
        assert_eq!(v.post, Some(1));
    }

    #[test]
    fn test_parse_devrelease() {
        let v = Pep440Version::parse("1.0.0.dev1").unwrap();
        assert_eq!(v.dev, Some(1));

        let v = Pep440Version::parse("1.0.0a1.dev2").unwrap();
        assert_eq!(v.pre, Some(PreRelease::Alpha(1)));
        assert_eq!(v.dev, Some(2));
    }

    #[test]
    fn test_parse_local() {
        let v = Pep440Version::parse("1.0.0+local").unwrap();
        assert_eq!(v.local, Some("local".to_string()));

        let v = Pep440Version::parse("1.0.0+ubuntu.1").unwrap();
        assert_eq!(v.local, Some("ubuntu.1".to_string()));
    }

    #[test]
    fn test_parse_complex() {
        let v = Pep440Version::parse("1!2.3.4a5.post6.dev7+local.8").unwrap();
        assert_eq!(v.epoch, 1);
        assert_eq!(v.release, vec![2, 3, 4]);
        assert_eq!(v.pre, Some(PreRelease::Alpha(5)));
        assert_eq!(v.post, Some(6));
        assert_eq!(v.dev, Some(7));
        assert_eq!(v.local, Some("local.8".to_string()));
    }

    #[test]
    fn test_display_roundtrip() {
        let cases = [
            "1.0.0",
            "1!2.0.0",
            "1.0.0a1",
            "1.0.0b2",
            "1.0.0rc3",
            "1.0.0.post1",
            "1.0.0.dev1",
            "1.0.0+local",
        ];

        for case in cases {
            let v = Pep440Version::parse(case).unwrap();
            let s = v.to_string();
            let v2 = Pep440Version::parse(&s).unwrap();
            assert_eq!(v, v2, "roundtrip failed for {}", case);
        }
    }

    #[test]
    fn test_ordering_basic() {
        let v1 = Pep440Version::parse("1.0.0").unwrap();
        let v2 = Pep440Version::parse("2.0.0").unwrap();
        assert!(v1 < v2);

        let v1 = Pep440Version::parse("1.0.0").unwrap();
        let v2 = Pep440Version::parse("1.1.0").unwrap();
        assert!(v1 < v2);

        let v1 = Pep440Version::parse("1.0.0").unwrap();
        let v2 = Pep440Version::parse("1.0.1").unwrap();
        assert!(v1 < v2);
    }

    #[test]
    fn test_ordering_epoch() {
        let v1 = Pep440Version::parse("2.0.0").unwrap();
        let v2 = Pep440Version::parse("1!1.0.0").unwrap();
        assert!(v1 < v2, "epoch should win");
    }

    #[test]
    fn test_ordering_prerelease() {
        // dev < alpha < beta < rc < release
        let dev = Pep440Version::parse("1.0.0.dev1").unwrap();
        let alpha = Pep440Version::parse("1.0.0a1").unwrap();
        let beta = Pep440Version::parse("1.0.0b1").unwrap();
        let rc = Pep440Version::parse("1.0.0rc1").unwrap();
        let release = Pep440Version::parse("1.0.0").unwrap();

        assert!(dev < alpha);
        assert!(alpha < beta);
        assert!(beta < rc);
        assert!(rc < release);
    }

    #[test]
    fn test_ordering_postrelease() {
        let release = Pep440Version::parse("1.0.0").unwrap();
        let post1 = Pep440Version::parse("1.0.0.post1").unwrap();
        let post2 = Pep440Version::parse("1.0.0.post2").unwrap();

        assert!(release < post1);
        assert!(post1 < post2);
    }

    #[test]
    fn test_ordering_local() {
        let v1 = Pep440Version::parse("1.0.0").unwrap();
        let v2 = Pep440Version::parse("1.0.0+local").unwrap();
        assert!(v1 < v2, "local version should be greater");

        let v1 = Pep440Version::parse("1.0.0+a").unwrap();
        let v2 = Pep440Version::parse("1.0.0+b").unwrap();
        assert!(v1 < v2);
    }

    #[test]
    fn test_is_prerelease() {
        assert!(Pep440Version::parse("1.0.0a1").unwrap().is_prerelease());
        assert!(Pep440Version::parse("1.0.0.dev1").unwrap().is_prerelease());
        assert!(!Pep440Version::parse("1.0.0").unwrap().is_prerelease());
        assert!(!Pep440Version::parse("1.0.0.post1").unwrap().is_prerelease());
    }

    #[test]
    fn test_normalize_v_prefix() {
        let v1 = Pep440Version::parse("v1.0.0").unwrap();
        let v2 = Pep440Version::parse("1.0.0").unwrap();
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_normalize_case() {
        let v1 = Pep440Version::parse("1.0.0RC1").unwrap();
        let v2 = Pep440Version::parse("1.0.0rc1").unwrap();
        assert_eq!(v1, v2);
    }
}
