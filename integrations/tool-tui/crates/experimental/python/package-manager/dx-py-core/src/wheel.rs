//! Wheel Tag Parser and Platform Detection
//!
//! Parses wheel filenames and detects platform compatibility.
//! Implements wheel tag matching per PEP 425 and PEP 427.

use std::cmp::Ordering;
use std::fmt;

/// Operating system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Windows,
    MacOs,
    Linux,
    FreeBsd,
    Unknown,
}

impl Os {
    /// Detect current OS
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Os::Windows;
        #[cfg(target_os = "macos")]
        return Os::MacOs;
        #[cfg(target_os = "linux")]
        return Os::Linux;
        #[cfg(target_os = "freebsd")]
        return Os::FreeBsd;
        #[cfg(not(any(
            target_os = "windows",
            target_os = "macos",
            target_os = "linux",
            target_os = "freebsd"
        )))]
        return Os::Unknown;
    }

    /// Get platform tag prefix
    pub fn tag_prefix(&self) -> &'static str {
        match self {
            Os::Windows => "win",
            Os::MacOs => "macosx",
            Os::Linux => "linux",
            Os::FreeBsd => "freebsd",
            Os::Unknown => "any",
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Os::Windows => write!(f, "Windows"),
            Os::MacOs => write!(f, "macOS"),
            Os::Linux => write!(f, "Linux"),
            Os::FreeBsd => write!(f, "FreeBSD"),
            Os::Unknown => write!(f, "Unknown"),
        }
    }
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    X86,
    X86_64,
    Aarch64,
    Arm,
    Universal2,
    Unknown,
}

impl Arch {
    /// Detect current architecture
    pub fn current() -> Self {
        #[cfg(target_arch = "x86")]
        return Arch::X86;
        #[cfg(target_arch = "x86_64")]
        return Arch::X86_64;
        #[cfg(target_arch = "aarch64")]
        return Arch::Aarch64;
        #[cfg(target_arch = "arm")]
        return Arch::Arm;
        #[cfg(not(any(
            target_arch = "x86",
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        return Arch::Unknown;
    }

    /// Get platform tag suffix
    pub fn tag_suffix(&self) -> &'static str {
        match self {
            Arch::X86 => "i686",
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
            Arch::Arm => "armv7l",
            Arch::Universal2 => "universal2",
            Arch::Unknown => "any",
        }
    }

    /// Check if this arch is compatible with a tag
    pub fn is_compatible_with(&self, tag: &str) -> bool {
        let tag = tag.to_lowercase();
        match self {
            Arch::X86_64 => {
                tag.contains("x86_64")
                    || tag.contains("amd64")
                    || tag == "any"
                    || tag.contains("universal")
            }
            Arch::Aarch64 => {
                tag.contains("aarch64")
                    || tag.contains("arm64")
                    || tag == "any"
                    || tag.contains("universal2")
            }
            Arch::X86 => tag.contains("i686") || tag.contains("i386") || tag == "any",
            Arch::Arm => tag.contains("armv7") || tag.contains("arm") || tag == "any",
            Arch::Universal2 => true,
            Arch::Unknown => tag == "any",
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.tag_suffix())
    }
}

/// Python implementation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PythonImpl {
    CPython,
    PyPy,
    GraalPy,
    Unknown,
}

impl PythonImpl {
    /// Get tag prefix
    pub fn tag_prefix(&self) -> &'static str {
        match self {
            PythonImpl::CPython => "cp",
            PythonImpl::PyPy => "pp",
            PythonImpl::GraalPy => "graalpy",
            PythonImpl::Unknown => "py",
        }
    }

    /// Parse from tag
    pub fn from_tag(tag: &str) -> Self {
        let tag = tag.to_lowercase();
        if tag.starts_with("cp") {
            PythonImpl::CPython
        } else if tag.starts_with("pp") {
            PythonImpl::PyPy
        } else if tag.starts_with("graalpy") {
            PythonImpl::GraalPy
        } else {
            PythonImpl::Unknown
        }
    }
}

/// Manylinux version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ManylinuxVersion {
    pub major: u32,
    pub minor: u32,
}

impl ManylinuxVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    /// Common manylinux versions
    pub const MANYLINUX1: Self = Self::new(2, 5);
    pub const MANYLINUX2010: Self = Self::new(2, 12);
    pub const MANYLINUX2014: Self = Self::new(2, 17);
    pub const MANYLINUX_2_24: Self = Self::new(2, 24);
    pub const MANYLINUX_2_28: Self = Self::new(2, 28);
    pub const MANYLINUX_2_31: Self = Self::new(2, 31);
    pub const MANYLINUX_2_34: Self = Self::new(2, 34);

    /// Parse from tag like "manylinux_2_17" or "manylinux2014"
    pub fn from_tag(tag: &str) -> Option<Self> {
        let tag = tag.to_lowercase();

        // Legacy names
        if tag.contains("manylinux1") {
            return Some(Self::MANYLINUX1);
        }
        if tag.contains("manylinux2010") {
            return Some(Self::MANYLINUX2010);
        }
        if tag.contains("manylinux2014") {
            return Some(Self::MANYLINUX2014);
        }

        // Modern format: manylinux_X_Y
        if let Some(rest) = tag.strip_prefix("manylinux_") {
            let parts: Vec<&str> = rest.split('_').collect();
            if parts.len() >= 2 {
                let major = parts[0].parse().ok()?;
                let minor = parts[1].parse().ok()?;
                return Some(Self::new(major, minor));
            }
        }

        None
    }

    /// Check if this version is compatible with a required version
    /// (this system can run wheels built for older glibc)
    pub fn is_compatible_with(&self, required: &Self) -> bool {
        if self.major != required.major {
            return self.major > required.major;
        }
        self.minor >= required.minor
    }
}

impl PartialOrd for ManylinuxVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ManylinuxVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => self.minor.cmp(&other.minor),
            ord => ord,
        }
    }
}

impl fmt::Display for ManylinuxVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "manylinux_{}_{}", self.major, self.minor)
    }
}

/// Platform environment for wheel compatibility checking
#[derive(Debug, Clone)]
pub struct PlatformEnvironment {
    pub os: Os,
    pub arch: Arch,
    pub python_impl: PythonImpl,
    pub python_version: (u32, u32),
    pub abi: String,
    pub manylinux: Option<ManylinuxVersion>,
    pub macos_version: Option<(u32, u32)>,
}

impl PlatformEnvironment {
    /// Detect current platform environment
    pub fn detect() -> Self {
        Self {
            os: Os::current(),
            arch: Arch::current(),
            python_impl: PythonImpl::CPython,
            python_version: (3, 12), // Default, should be detected
            abi: String::from("cp312"),
            manylinux: Self::detect_manylinux(),
            macos_version: Self::detect_macos_version(),
        }
    }

    /// Create with specific Python version
    pub fn with_python(mut self, major: u32, minor: u32) -> Self {
        self.python_version = (major, minor);
        self.abi = format!("cp{}{}", major, minor);
        self
    }

    /// Detect manylinux compatibility
    fn detect_manylinux() -> Option<ManylinuxVersion> {
        #[cfg(target_os = "linux")]
        {
            // Try to detect glibc version
            // For now, assume modern systems support manylinux_2_17
            Some(ManylinuxVersion::MANYLINUX2014)
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    /// Detect macOS version
    fn detect_macos_version() -> Option<(u32, u32)> {
        #[cfg(target_os = "macos")]
        {
            // Default to 10.9 (minimum for most wheels)
            Some((10, 9))
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    /// Get compatible Python tags
    pub fn python_tags(&self) -> Vec<String> {
        let (major, minor) = self.python_version;
        let impl_prefix = self.python_impl.tag_prefix();

        let mut tags = Vec::new();

        // Implementation-specific tags (cp312, cp311, ...)
        for m in (0..=minor).rev() {
            tags.push(format!("{}{}{}", impl_prefix, major, m));
        }

        // Generic Python tags (py3, py312, ...)
        tags.push(format!("py{}", major));
        for m in (0..=minor).rev() {
            tags.push(format!("py{}{}", major, m));
        }

        // py2.py3 universal
        tags.push("py2.py3".to_string());

        tags
    }

    /// Get compatible ABI tags
    pub fn abi_tags(&self) -> Vec<String> {
        let (major, minor) = self.python_version;
        let impl_prefix = self.python_impl.tag_prefix();

        let mut tags = Vec::new();

        // Implementation-specific ABI
        tags.push(format!("{}{}{}", impl_prefix, major, minor));

        // ABI3 (stable ABI)
        tags.push("abi3".to_string());

        // None (pure Python)
        tags.push("none".to_string());

        tags
    }

    /// Get compatible platform tags
    pub fn platform_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();

        match self.os {
            Os::Windows => match self.arch {
                Arch::X86_64 => tags.push("win_amd64".to_string()),
                Arch::X86 => tags.push("win32".to_string()),
                Arch::Aarch64 => tags.push("win_arm64".to_string()),
                _ => {}
            },
            Os::MacOs => {
                if let Some((major, minor)) = self.macos_version {
                    let arch_tag = match self.arch {
                        Arch::X86_64 => "x86_64",
                        Arch::Aarch64 => "arm64",
                        Arch::Universal2 => "universal2",
                        _ => "x86_64",
                    };

                    // Add tags for this version and older
                    for m in (0..=minor).rev() {
                        tags.push(format!("macosx_{}_{}_{}", major, m, arch_tag));
                    }

                    // Universal tags
                    if self.arch == Arch::Aarch64 {
                        tags.push(format!("macosx_{}_0_universal2", major));
                    }
                }
            }
            Os::Linux => {
                let arch = self.arch.tag_suffix();

                // Manylinux tags (prefer newer)
                if let Some(ref ml) = self.manylinux {
                    // Add compatible manylinux versions (newer to older)
                    let versions = [
                        ManylinuxVersion::MANYLINUX_2_34,
                        ManylinuxVersion::MANYLINUX_2_31,
                        ManylinuxVersion::MANYLINUX_2_28,
                        ManylinuxVersion::MANYLINUX_2_24,
                        ManylinuxVersion::MANYLINUX2014,
                        ManylinuxVersion::MANYLINUX2010,
                        ManylinuxVersion::MANYLINUX1,
                    ];

                    for v in versions {
                        if ml.is_compatible_with(&v) {
                            tags.push(format!("manylinux_{}_{}_{}", v.major, v.minor, arch));
                        }
                    }

                    // Legacy names
                    if ml.is_compatible_with(&ManylinuxVersion::MANYLINUX2014) {
                        tags.push(format!("manylinux2014_{}", arch));
                    }
                    if ml.is_compatible_with(&ManylinuxVersion::MANYLINUX2010) {
                        tags.push(format!("manylinux2010_{}", arch));
                    }
                    if ml.is_compatible_with(&ManylinuxVersion::MANYLINUX1) {
                        tags.push(format!("manylinux1_{}", arch));
                    }
                }

                // Linux generic
                tags.push(format!("linux_{}", arch));
            }
            _ => {}
        }

        // Universal
        tags.push("any".to_string());

        tags
    }
}

/// Parsed wheel filename
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WheelTag {
    /// Package name (normalized)
    pub name: String,
    /// Version string
    pub version: String,
    /// Build tag (optional)
    pub build: Option<String>,
    /// Python tags (e.g., ["cp312", "cp311"])
    pub python_tags: Vec<String>,
    /// ABI tags (e.g., ["cp312", "abi3", "none"])
    pub abi_tags: Vec<String>,
    /// Platform tags (e.g., ["manylinux_2_17_x86_64", "linux_x86_64"])
    pub platform_tags: Vec<String>,
}

/// Wheel parse error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WheelParseError {
    InvalidFormat(String),
    MissingComponent(String),
}

impl fmt::Display for WheelParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WheelParseError::InvalidFormat(s) => write!(f, "invalid wheel format: {}", s),
            WheelParseError::MissingComponent(s) => write!(f, "missing component: {}", s),
        }
    }
}

impl std::error::Error for WheelParseError {}

impl WheelTag {
    /// Parse a wheel filename
    ///
    /// Format: {distribution}-{version}(-{build tag})?-{python tag}-{abi tag}-{platform tag}.whl
    pub fn parse(filename: &str) -> Result<Self, WheelParseError> {
        let filename = filename.strip_suffix(".whl").unwrap_or(filename);
        let parts: Vec<&str> = filename.split('-').collect();

        if parts.len() < 5 {
            return Err(WheelParseError::InvalidFormat(format!(
                "expected at least 5 parts, got {}",
                parts.len()
            )));
        }

        // Last 3 parts are always python-abi-platform
        let platform_idx = parts.len() - 1;
        let abi_idx = parts.len() - 2;
        let python_idx = parts.len() - 3;

        // Check for build tag (6 parts means build tag present)
        let (name, version, build) = if parts.len() >= 6 {
            // Could have build tag - check if parts[2] looks like a build tag
            // Build tags are numeric or start with a number
            let potential_build = parts[2];
            if potential_build.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                (parts[0].to_string(), parts[1].to_string(), Some(potential_build.to_string()))
            } else {
                // No build tag, name might have hyphens
                let name_parts = &parts[..parts.len() - 4];
                (name_parts.join("-"), parts[parts.len() - 4].to_string(), None)
            }
        } else {
            (parts[0].to_string(), parts[1].to_string(), None)
        };

        // Parse tags (can have multiple separated by .)
        let python_tags: Vec<String> = parts[python_idx].split('.').map(String::from).collect();
        let abi_tags: Vec<String> = parts[abi_idx].split('.').map(String::from).collect();
        let platform_tags: Vec<String> = parts[platform_idx].split('.').map(String::from).collect();

        Ok(Self {
            name: normalize_name(&name),
            version,
            build,
            python_tags,
            abi_tags,
            platform_tags,
        })
    }

    /// Check if this wheel is compatible with the given environment
    pub fn is_compatible(&self, env: &PlatformEnvironment) -> bool {
        self.python_compatible(env) && self.abi_compatible(env) && self.platform_compatible(env)
    }

    /// Check Python compatibility
    fn python_compatible(&self, env: &PlatformEnvironment) -> bool {
        let env_tags = env.python_tags();
        self.python_tags.iter().any(|t| env_tags.iter().any(|e| tags_match(t, e)))
    }

    /// Check ABI compatibility
    fn abi_compatible(&self, env: &PlatformEnvironment) -> bool {
        let env_tags = env.abi_tags();
        self.abi_tags.iter().any(|t| env_tags.iter().any(|e| tags_match(t, e)))
    }

    /// Check platform compatibility
    fn platform_compatible(&self, env: &PlatformEnvironment) -> bool {
        let env_tags = env.platform_tags();
        self.platform_tags.iter().any(|t| env_tags.iter().any(|e| tags_match(t, e)))
    }

    /// Calculate specificity score (higher = more specific = preferred)
    pub fn specificity_score(&self, env: &PlatformEnvironment) -> u32 {
        let mut score = 0u32;

        // Python tag specificity
        for tag in &self.python_tags {
            if tag.starts_with("cp") {
                score += 100; // CPython specific
            } else if tag.starts_with("py") && tag.len() > 3 {
                score += 50; // py3X specific
            } else if tag == "py3" || tag == "py2.py3" {
                score += 10; // Generic
            }
        }

        // ABI specificity
        for tag in &self.abi_tags {
            if tag.starts_with("cp") {
                score += 100; // CPython ABI
            } else if tag == "abi3" {
                score += 50; // Stable ABI
            } else if tag == "none" {
                score += 10; // Pure Python
            }
        }

        // Platform specificity
        for tag in &self.platform_tags {
            if tag == "any" {
                score += 10;
            } else if tag.contains("manylinux") {
                // Prefer newer manylinux
                if let Some(ml) = ManylinuxVersion::from_tag(tag) {
                    score += 100 + ml.minor; // Higher minor = newer = better
                }
            } else if tag.starts_with("win") || tag.starts_with("macosx") {
                score += 200; // Platform-specific
            } else if tag.starts_with("linux") {
                score += 150;
            }
        }

        // Bonus for matching current platform exactly
        if self
            .platform_tags
            .iter()
            .any(|t| env.platform_tags().first().map(|e| tags_match(t, e)).unwrap_or(false))
        {
            score += 500;
        }

        score
    }

    /// Format as wheel filename
    pub fn to_filename(&self) -> String {
        let python = self.python_tags.join(".");
        let abi = self.abi_tags.join(".");
        let platform = self.platform_tags.join(".");

        if let Some(ref build) = self.build {
            format!("{}-{}-{}-{}-{}-{}.whl", self.name, self.version, build, python, abi, platform)
        } else {
            format!("{}-{}-{}-{}-{}.whl", self.name, self.version, python, abi, platform)
        }
    }
}

/// Normalize package name (PEP 503)
fn normalize_name(name: &str) -> String {
    name.to_lowercase().replace(['-', '.', '_'], "_")
}

/// Check if two tags match (case-insensitive)
fn tags_match(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Select the best wheel from a list of compatible wheels
pub fn select_best_wheel<'a>(
    wheels: &'a [WheelTag],
    env: &PlatformEnvironment,
) -> Option<&'a WheelTag> {
    wheels
        .iter()
        .filter(|w| w.is_compatible(env))
        .max_by_key(|w| w.specificity_score(env))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_wheel() {
        let wheel = WheelTag::parse("requests-2.31.0-py3-none-any.whl").unwrap();
        assert_eq!(wheel.name, "requests");
        assert_eq!(wheel.version, "2.31.0");
        assert_eq!(wheel.python_tags, vec!["py3"]);
        assert_eq!(wheel.abi_tags, vec!["none"]);
        assert_eq!(wheel.platform_tags, vec!["any"]);
    }

    #[test]
    fn test_parse_cpython_wheel() {
        let wheel = WheelTag::parse("numpy-1.26.0-cp312-cp312-manylinux_2_17_x86_64.whl").unwrap();
        assert_eq!(wheel.name, "numpy");
        assert_eq!(wheel.version, "1.26.0");
        assert_eq!(wheel.python_tags, vec!["cp312"]);
        assert_eq!(wheel.abi_tags, vec!["cp312"]);
        assert_eq!(wheel.platform_tags, vec!["manylinux_2_17_x86_64"]);
    }

    #[test]
    fn test_parse_multi_tag_wheel() {
        let wheel = WheelTag::parse("six-1.16.0-py2.py3-none-any.whl").unwrap();
        assert_eq!(wheel.python_tags, vec!["py2", "py3"]);
    }

    #[test]
    fn test_parse_windows_wheel() {
        let wheel = WheelTag::parse("pywin32-306-cp312-cp312-win_amd64.whl").unwrap();
        assert_eq!(wheel.platform_tags, vec!["win_amd64"]);
    }

    #[test]
    fn test_parse_macos_wheel() {
        let wheel =
            WheelTag::parse("cryptography-41.0.0-cp312-cp312-macosx_10_12_x86_64.whl").unwrap();
        assert_eq!(wheel.platform_tags, vec!["macosx_10_12_x86_64"]);
    }

    #[test]
    fn test_wheel_compatibility() {
        let env = PlatformEnvironment::detect().with_python(3, 12);

        // Pure Python wheel should be compatible everywhere
        let pure = WheelTag::parse("requests-2.31.0-py3-none-any.whl").unwrap();
        assert!(pure.is_compatible(&env));
    }

    #[test]
    fn test_wheel_specificity() {
        let env = PlatformEnvironment::detect().with_python(3, 12);

        let pure = WheelTag::parse("pkg-1.0.0-py3-none-any.whl").unwrap();
        let specific = WheelTag::parse("pkg-1.0.0-cp312-cp312-manylinux_2_17_x86_64.whl").unwrap();

        // Platform-specific should have higher score
        assert!(specific.specificity_score(&env) > pure.specificity_score(&env));
    }

    #[test]
    fn test_manylinux_version_parsing() {
        assert_eq!(
            ManylinuxVersion::from_tag("manylinux_2_17_x86_64"),
            Some(ManylinuxVersion::new(2, 17))
        );
        assert_eq!(
            ManylinuxVersion::from_tag("manylinux2014_x86_64"),
            Some(ManylinuxVersion::MANYLINUX2014)
        );
        assert_eq!(
            ManylinuxVersion::from_tag("manylinux1_x86_64"),
            Some(ManylinuxVersion::MANYLINUX1)
        );
    }

    #[test]
    fn test_manylinux_compatibility() {
        let system = ManylinuxVersion::MANYLINUX2014; // glibc 2.17

        // Can run wheels built for older glibc
        assert!(system.is_compatible_with(&ManylinuxVersion::MANYLINUX1));
        assert!(system.is_compatible_with(&ManylinuxVersion::MANYLINUX2010));
        assert!(system.is_compatible_with(&ManylinuxVersion::MANYLINUX2014));

        // Cannot run wheels built for newer glibc
        assert!(!system.is_compatible_with(&ManylinuxVersion::MANYLINUX_2_24));
    }

    #[test]
    fn test_select_best_wheel() {
        let env = PlatformEnvironment {
            os: Os::Linux,
            arch: Arch::X86_64,
            python_impl: PythonImpl::CPython,
            python_version: (3, 12),
            abi: "cp312".to_string(),
            manylinux: Some(ManylinuxVersion::MANYLINUX2014),
            macos_version: None,
        };

        let wheels = vec![
            WheelTag::parse("pkg-1.0.0-py3-none-any.whl").unwrap(),
            WheelTag::parse("pkg-1.0.0-cp312-cp312-manylinux_2_17_x86_64.whl").unwrap(),
            WheelTag::parse("pkg-1.0.0-cp312-abi3-manylinux_2_17_x86_64.whl").unwrap(),
        ];

        let best = select_best_wheel(&wheels, &env).unwrap();
        // Should prefer cp312-cp312 over abi3 over pure python
        assert_eq!(best.abi_tags, vec!["cp312"]);
    }

    #[test]
    fn test_wheel_to_filename() {
        let wheel = WheelTag {
            name: "requests".to_string(),
            version: "2.31.0".to_string(),
            build: None,
            python_tags: vec!["py3".to_string()],
            abi_tags: vec!["none".to_string()],
            platform_tags: vec!["any".to_string()],
        };

        assert_eq!(wheel.to_filename(), "requests-2.31.0-py3-none-any.whl");
    }
}
