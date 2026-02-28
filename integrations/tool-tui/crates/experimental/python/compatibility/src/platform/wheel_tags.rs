//! Wheel tag generation
//!
//! Generates compatible wheel tags for package selection.

use super::{Architecture, Libc, Os, Platform};
use crate::runtime::PythonRuntime;

/// Wheel tag (python-abi-platform)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WheelTag {
    /// Python tag (e.g., "cp312", "py3")
    pub python: String,
    /// ABI tag (e.g., "cp312", "abi3", "none")
    pub abi: String,
    /// Platform tag (e.g., "manylinux_2_17_x86_64", "win_amd64")
    pub platform: String,
    /// Priority (lower is better)
    pub priority: u32,
}

impl WheelTag {
    /// Create a new wheel tag
    pub fn new(
        python: impl Into<String>,
        abi: impl Into<String>,
        platform: impl Into<String>,
        priority: u32,
    ) -> Self {
        Self {
            python: python.into(),
            abi: abi.into(),
            platform: platform.into(),
            priority,
        }
    }

    /// Get the full tag string
    pub fn as_string(&self) -> String {
        format!("{}-{}-{}", self.python, self.abi, self.platform)
    }
}

/// Generates wheel tags for package selection
pub struct WheelTagGenerator {
    platform: Platform,
    python_version: (u8, u8),
    abi_tag: String,
}

impl WheelTagGenerator {
    /// Create generator for platform and Python
    pub fn new(platform: Platform, python: &PythonRuntime) -> Self {
        Self {
            platform,
            python_version: (python.version.major, python.version.minor),
            abi_tag: python.capabilities.abi_tag.clone(),
        }
    }

    /// Create generator with explicit version
    pub fn with_version(platform: Platform, major: u8, minor: u8) -> Self {
        Self {
            platform,
            python_version: (major, minor),
            abi_tag: format!("cp{}{}", major, minor),
        }
    }

    /// Generate all compatible tags in priority order
    pub fn generate_tags(&self) -> Vec<WheelTag> {
        let mut tags = Vec::new();
        let mut priority = 0u32;

        // CPython-specific tags (highest priority)
        tags.extend(self.generate_cpython_tags(&mut priority));

        // ABI3 tags
        tags.extend(self.generate_abi3_tags(&mut priority));

        // Pure Python tags (lowest priority)
        tags.extend(self.generate_pure_python_tags(&mut priority));

        tags
    }

    /// Generate CPython-specific tags
    fn generate_cpython_tags(&self, priority: &mut u32) -> Vec<WheelTag> {
        let mut tags = Vec::new();
        let (major, minor) = self.python_version;
        let python_tag = format!("cp{}{}", major, minor);
        let abi_tag = if self.abi_tag.is_empty() {
            format!("cp{}{}", major, minor)
        } else {
            self.abi_tag.clone()
        };

        // Platform-specific tags
        for platform_tag in self.generate_platform_tags() {
            tags.push(WheelTag::new(&python_tag, &abi_tag, &platform_tag, *priority));
            *priority += 1;
        }

        // None ABI with platform
        for platform_tag in self.generate_platform_tags() {
            tags.push(WheelTag::new(&python_tag, "none", &platform_tag, *priority));
            *priority += 1;
        }

        tags
    }

    /// Generate ABI3 tags
    fn generate_abi3_tags(&self, priority: &mut u32) -> Vec<WheelTag> {
        let mut tags = Vec::new();
        let (major, minor) = self.python_version;

        // ABI3 tags for all compatible Python versions
        for m in (2..=minor).rev() {
            let python_tag = format!("cp{}{}", major, m);
            for platform_tag in self.generate_platform_tags() {
                tags.push(WheelTag::new(&python_tag, "abi3", &platform_tag, *priority));
                *priority += 1;
            }
        }

        tags
    }

    /// Generate pure Python tags
    fn generate_pure_python_tags(&self, priority: &mut u32) -> Vec<WheelTag> {
        let mut tags = Vec::new();
        let (major, minor) = self.python_version;

        // py3X-none-any
        for m in (0..=minor).rev() {
            let python_tag = format!("py{}{}", major, m);
            tags.push(WheelTag::new(&python_tag, "none", "any", *priority));
            *priority += 1;
        }

        // py3-none-any
        tags.push(WheelTag::new(format!("py{}", major), "none", "any", *priority));
        *priority += 1;

        tags
    }

    /// Generate platform tags in priority order
    fn generate_platform_tags(&self) -> Vec<String> {
        let mut tags = Vec::new();
        let arch = self.platform.arch.wheel_platform();

        match &self.platform.os {
            Os::Windows => {
                tags.push(self.platform.wheel_platform_tag());
            }
            Os::MacOs => {
                // macOS universal and specific tags
                let mac_arch = match &self.platform.arch {
                    Architecture::X86_64 => "x86_64",
                    Architecture::Aarch64 => "arm64",
                    _ => arch,
                };

                // Specific version tags (10.9 through 14.0)
                for major in (9..=14).rev() {
                    tags.push(format!("macosx_10_{}_universal2", major));
                    tags.push(format!("macosx_10_{}_{}", major, mac_arch));
                }

                // macOS 11+ for ARM
                if matches!(self.platform.arch, Architecture::Aarch64) {
                    for major in (0..=4).rev() {
                        tags.push(format!("macosx_11_{}_arm64", major));
                    }
                }
            }
            Os::Linux => {
                // manylinux tags
                if let Some(Libc::Glibc { major, minor }) = &self.platform.libc {
                    // manylinux_x_y tags
                    for m in (17..=*minor).rev() {
                        tags.push(format!("manylinux_{}_{}", major, m));
                    }
                    // Legacy manylinux tags
                    if *minor >= 17 {
                        tags.push(format!("manylinux2014_{}", arch));
                    }
                    if *minor >= 12 {
                        tags.push(format!("manylinux2010_{}", arch));
                    }
                    if *minor >= 5 {
                        tags.push(format!("manylinux1_{}", arch));
                    }
                }

                // musllinux tags
                if let Some(Libc::Musl { major, minor }) = &self.platform.libc {
                    for m in (0..=*minor).rev() {
                        tags.push(format!("musllinux_{}_{}", major, m));
                    }
                }

                // Generic linux tag
                tags.push(format!("linux_{}", arch));
            }
            _ => {
                tags.push(self.platform.wheel_platform_tag());
            }
        }

        // Add architecture suffix to manylinux/musllinux tags
        let arch_suffix = format!("_{}", arch);
        tags = tags
            .iter()
            .map(|t| {
                if (t.starts_with("manylinux") || t.starts_with("musllinux"))
                    && !t.ends_with(&arch_suffix)
                {
                    format!("{}{}", t, arch_suffix)
                } else {
                    t.clone()
                }
            })
            .collect();

        tags
    }

    /// Check if a wheel tag is compatible
    pub fn is_compatible(&self, tag: &WheelTag) -> bool {
        self.generate_tags()
            .iter()
            .any(|t| t.python == tag.python && t.abi == tag.abi && t.platform == tag.platform)
    }

    /// Select best wheel from candidates
    pub fn select_best<'a>(&self, tags: &'a [WheelTag]) -> Option<&'a WheelTag> {
        let compatible_tags = self.generate_tags();

        tags.iter()
            .filter_map(|tag| {
                compatible_tags
                    .iter()
                    .find(|t| {
                        t.python == tag.python && t.abi == tag.abi && t.platform == tag.platform
                    })
                    .map(|t| (tag, t.priority))
            })
            .min_by_key(|(_, priority)| *priority)
            .map(|(tag, _)| tag)
    }
}
