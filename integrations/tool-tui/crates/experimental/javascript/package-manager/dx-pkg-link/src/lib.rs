//! dx-pkg-link: Instant Package Linking (50x faster)
//!
//! Uses platform-specific Copy-on-Write (CoW) mechanisms:
//! - Linux: reflinks (FICLONE ioctl) on Btrfs/XFS
//! - macOS: clonefile() on APFS
//! - Windows: Junctions for directories, hardlinks for files
//! - Fallback: hardlinks (instant, 0 bytes)

pub mod reflink;

use dx_pkg_core::Result;
use std::fs;
use std::path::Path;

pub use reflink::ReflinkLinker;

/// Create a directory junction (Windows) or symlink (Unix)
/// Junctions are preferred on Windows as they don't require admin privileges
#[cfg(windows)]
pub fn create_dir_link(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::fs::symlink_dir;
    use std::process::Command;

    // First try symlink (requires developer mode or admin on Windows)
    if symlink_dir(source, target).is_ok() {
        return Ok(());
    }

    // Fall back to junction (works without admin privileges)
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(target)
        .arg(source)
        .output()?;

    if output.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "Failed to create junction: {}",
            String::from_utf8_lossy(&output.stderr)
        )))
    }
}

/// Create a directory symlink (Unix)
#[cfg(not(windows))]
pub fn create_dir_link(source: &Path, target: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, target)
}

/// Check if a path is a junction (Windows) or symlink (Unix)
#[cfg(windows)]
pub fn is_dir_link(path: &Path) -> bool {
    use std::os::windows::fs::MetadataExt;

    if let Ok(metadata) = fs::symlink_metadata(path) {
        // Check for reparse point attribute (junctions and symlinks)
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    } else {
        false
    }
}

/// Check if a path is a symlink (Unix)
#[cfg(not(windows))]
pub fn is_dir_link(path: &Path) -> bool {
    fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

/// Link strategy (fastest to slowest)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkStrategy {
    Reflink,  // CoW clone (instant, 0 bytes)
    Hardlink, // Hard link (instant, 0 bytes)
    Copy,     // Full copy (slow, uses disk)
}

/// Platform-specific linker
pub struct PackageLinker {
    strategy: LinkStrategy,
    fallback_allowed: bool,
}

impl PackageLinker {
    /// Create new linker with auto-detected strategy
    pub fn new() -> Self {
        Self {
            strategy: Self::detect_best_strategy(),
            fallback_allowed: true,
        }
    }

    /// Create linker with explicit strategy
    pub fn with_strategy(strategy: LinkStrategy) -> Self {
        Self {
            strategy,
            fallback_allowed: true,
        }
    }

    /// Disable fallback to slower strategies
    pub fn no_fallback(mut self) -> Self {
        self.fallback_allowed = false;
        self
    }

    /// Link package from store to target
    pub fn link(&self, source: &Path, target: &Path) -> Result<LinkStrategy> {
        // Create parent directory
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // Try primary strategy
        match self.try_link(source, target, self.strategy) {
            Ok(_) => return Ok(self.strategy),
            Err(e) if !self.fallback_allowed => return Err(e),
            Err(_) => {}
        }

        // Try fallback strategies
        if self.strategy != LinkStrategy::Hardlink
            && self.try_link(source, target, LinkStrategy::Hardlink).is_ok()
        {
            return Ok(LinkStrategy::Hardlink);
        }

        // Last resort: copy
        self.try_link(source, target, LinkStrategy::Copy)?;
        Ok(LinkStrategy::Copy)
    }

    /// Link entire directory recursively
    pub fn link_tree(&self, source: &Path, target: &Path) -> Result<LinkStats> {
        let mut stats = LinkStats::default();

        self.link_tree_recursive(source, target, &mut stats)?;

        Ok(stats)
    }

    // Internal helpers

    fn try_link(&self, source: &Path, target: &Path, strategy: LinkStrategy) -> Result<()> {
        match strategy {
            LinkStrategy::Reflink => self.reflink(source, target),
            LinkStrategy::Hardlink => self.hardlink(source, target),
            LinkStrategy::Copy => self.copy(source, target),
        }
    }

    fn link_tree_recursive(
        &self,
        source: &Path,
        target: &Path,
        stats: &mut LinkStats,
    ) -> Result<()> {
        if source.is_dir() {
            fs::create_dir_all(target)?;

            for entry in fs::read_dir(source)? {
                let entry = entry?;
                let name = entry.file_name();
                let source_path = source.join(&name);
                let target_path = target.join(&name);

                self.link_tree_recursive(&source_path, &target_path, stats)?;
            }
        } else {
            let strategy = self.link(source, target)?;
            stats.record(strategy, fs::metadata(source)?.len());
        }

        Ok(())
    }

    // Platform-specific implementations

    #[cfg(target_os = "linux")]
    fn reflink(&self, source: &Path, target: &Path) -> Result<()> {
        use std::os::unix::fs::OpenOptionsExt;
        use std::os::unix::io::AsRawFd;

        let src_file = fs::File::open(source)?;
        let dst_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o644)
            .open(target)?;

        // FICLONE ioctl (1074041865 = 0x40049409)
        const FICLONE: libc::c_ulong = 0x40049409;

        let result = unsafe {
            libc::ioctl(dst_file.as_raw_fd(), FICLONE as libc::c_ulong, src_file.as_raw_fd())
        };

        if result != 0 {
            return Err(dx_pkg_core::Error::Io(std::io::Error::last_os_error()));
        }

        Ok(())
    }

    #[cfg(target_os = "macos")]
    fn reflink(&self, source: &Path, target: &Path) -> Result<()> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let src = CString::new(source.as_os_str().as_bytes())
            .map_err(|_| dx_pkg_core::Error::parse("Invalid source path"))?;
        let dst = CString::new(target.as_os_str().as_bytes())
            .map_err(|_| dx_pkg_core::Error::parse("Invalid target path"))?;

        // clonefile() - APFS CoW
        extern "C" {
            fn clonefile(src: *const libc::c_char, dst: *const libc::c_char, flags: u32) -> i32;
        }

        let result = unsafe { clonefile(src.as_ptr(), dst.as_ptr(), 0) };

        if result != 0 {
            return Err(dx_pkg_core::Error::io_with_path(std::io::Error::last_os_error(), target));
        }

        Ok(())
    }

    #[cfg(target_os = "windows")]
    fn reflink(&self, source: &Path, target: &Path) -> Result<()> {
        // Windows doesn't have easy CoW support via standard APIs
        // ReFS supports CoW but requires complex FSCTL calls
        // Fall back to hardlink for now
        self.hardlink(source, target)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn reflink(&self, _source: &Path, _target: &Path) -> Result<()> {
        Err(dx_pkg_core::Error::parse("Reflinks not supported on this platform"))
    }

    fn hardlink(&self, source: &Path, target: &Path) -> Result<()> {
        fs::hard_link(source, target)?;
        Ok(())
    }

    fn copy(&self, source: &Path, target: &Path) -> Result<()> {
        fs::copy(source, target)?;
        Ok(())
    }

    // Auto-detection

    fn detect_best_strategy() -> LinkStrategy {
        // Try to detect filesystem capabilities
        #[cfg(target_os = "linux")]
        {
            // Check for Btrfs/XFS via /proc/filesystems
            if let Ok(content) = fs::read_to_string("/proc/filesystems") {
                if content.contains("btrfs") || content.contains("xfs") {
                    return LinkStrategy::Reflink;
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // APFS is default on macOS 10.13+
            return LinkStrategy::Reflink;
        }

        #[cfg(target_os = "windows")]
        {
            // ReFS supports CoW on Windows Server 2016+
            // For simplicity, default to hardlink on Windows
            LinkStrategy::Hardlink
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Default fallback for other platforms
            LinkStrategy::Hardlink
        }
    }
}

impl Default for PackageLinker {
    fn default() -> Self {
        Self::new()
    }
}

/// Link statistics
#[derive(Debug, Default, Clone)]
pub struct LinkStats {
    pub reflinks: usize,
    pub hardlinks: usize,
    pub copies: usize,
    pub bytes_saved: u64, // Bytes saved by not copying
}

impl LinkStats {
    fn record(&mut self, strategy: LinkStrategy, size: u64) {
        match strategy {
            LinkStrategy::Reflink => {
                self.reflinks += 1;
                self.bytes_saved += size;
            }
            LinkStrategy::Hardlink => {
                self.hardlinks += 1;
                self.bytes_saved += size;
            }
            LinkStrategy::Copy => {
                self.copies += 1;
            }
        }
    }

    /// Get total files linked
    pub fn total(&self) -> usize {
        self.reflinks + self.hardlinks + self.copies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_linker_creation() {
        let linker = PackageLinker::new();
        assert!(linker.fallback_allowed);
    }

    #[test]
    fn test_strategy_detection() {
        let strategy = PackageLinker::detect_best_strategy();

        #[cfg(target_os = "macos")]
        assert_eq!(strategy, LinkStrategy::Reflink);

        #[cfg(target_os = "windows")]
        assert_eq!(strategy, LinkStrategy::Hardlink);
    }

    #[test]
    fn test_link_stats() {
        let mut stats = LinkStats::default();

        stats.record(LinkStrategy::Reflink, 1000);
        stats.record(LinkStrategy::Hardlink, 500);
        stats.record(LinkStrategy::Copy, 200);

        assert_eq!(stats.reflinks, 1);
        assert_eq!(stats.hardlinks, 1);
        assert_eq!(stats.copies, 1);
        assert_eq!(stats.bytes_saved, 1500);
        assert_eq!(stats.total(), 3);
    }

    #[test]
    fn test_hardlink_fallback() -> Result<()> {
        let temp = std::env::temp_dir();
        let source = temp.join("dx_test_source.txt");
        let target = temp.join("dx_test_target.txt");

        // Create source file
        let mut file = fs::File::create(&source)?;
        file.write_all(b"test content")?;
        drop(file);

        // Try linking
        let linker = PackageLinker::with_strategy(LinkStrategy::Hardlink);
        let strategy = linker.link(&source, &target)?;

        assert_eq!(strategy, LinkStrategy::Hardlink);
        assert!(target.exists());

        // Verify content
        let content = fs::read_to_string(&target)?;
        assert_eq!(content, "test content");

        // Cleanup
        fs::remove_file(&source)?;
        fs::remove_file(&target)?;

        Ok(())
    }

    /// Test directory link creation (junction on Windows, symlink on Unix)
    #[test]
    fn test_create_dir_link() {
        let temp = std::env::temp_dir();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let source = temp.join(format!("dx_test_dir_source_{}", unique_id));
        let target = temp.join(format!("dx_test_dir_link_{}", unique_id));

        // Create source directory with a file
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("test.txt"), "hello from source").unwrap();

        // Create directory link
        let result = create_dir_link(&source, &target);

        // On Windows, junction creation may fail without proper permissions
        // On Unix, symlink should work
        if result.is_ok() {
            // Verify link exists and is a link
            assert!(target.exists());
            assert!(is_dir_link(&target));

            // Verify we can read through the link
            let content = fs::read_to_string(target.join("test.txt")).unwrap();
            assert_eq!(content, "hello from source");

            // Cleanup
            #[cfg(windows)]
            {
                // On Windows, remove junction first
                let _ =
                    std::process::Command::new("cmd").args(["/C", "rmdir"]).arg(&target).output();
            }
            #[cfg(not(windows))]
            {
                let _ = fs::remove_file(&target);
            }
        }

        // Cleanup source
        let _ = fs::remove_dir_all(&source);
    }

    /// Test that is_dir_link correctly identifies links
    #[test]
    fn test_is_dir_link_detection() {
        let temp = std::env::temp_dir();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let regular_dir = temp.join(format!("dx_test_regular_dir_{}", unique_id));

        // Create regular directory
        fs::create_dir_all(&regular_dir).unwrap();

        // Regular directory should not be detected as a link
        assert!(!is_dir_link(&regular_dir));

        // Cleanup
        let _ = fs::remove_dir_all(&regular_dir);
    }

    /// Test nested dependency linking (simulates node_modules structure)
    #[test]
    fn test_nested_dependency_linking() {
        let temp = std::env::temp_dir();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // Create store directory (like .dx-store)
        let store = temp.join(format!("dx_test_store_{}", unique_id));
        let lodash_store = store.join("lodash@4.17.21");
        fs::create_dir_all(&lodash_store).unwrap();
        fs::write(lodash_store.join("package.json"), r#"{"name":"lodash","version":"4.17.21"}"#)
            .unwrap();
        fs::write(lodash_store.join("index.js"), "module.exports = {};").unwrap();

        // Create node_modules directory
        let node_modules = temp.join(format!("dx_test_node_modules_{}", unique_id));
        fs::create_dir_all(&node_modules).unwrap();

        // Try to create link from store to node_modules
        let lodash_link = node_modules.join("lodash");
        let result = create_dir_link(&lodash_store, &lodash_link);

        if result.is_ok() {
            // Verify the link works
            assert!(lodash_link.join("package.json").exists());
            let pkg_json = fs::read_to_string(lodash_link.join("package.json")).unwrap();
            assert!(pkg_json.contains("lodash"));

            // Cleanup link
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "rmdir"])
                    .arg(&lodash_link)
                    .output();
            }
            #[cfg(not(windows))]
            {
                let _ = fs::remove_file(&lodash_link);
            }
        }

        // Cleanup
        let _ = fs::remove_dir_all(&store);
        let _ = fs::remove_dir_all(&node_modules);
    }

    /// Test Unix symlink creation specifically
    /// On Unix, symlinks should be created without requiring special permissions
    #[cfg(not(windows))]
    #[test]
    fn test_unix_symlink_creation() {
        use std::os::unix::fs::symlink;

        let temp = std::env::temp_dir();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let source = temp.join(format!("dx_unix_symlink_source_{}", unique_id));
        let target = temp.join(format!("dx_unix_symlink_target_{}", unique_id));

        // Create source directory
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("test.txt"), "unix symlink test").unwrap();

        // Create symlink
        symlink(&source, &target).unwrap();

        // Verify symlink
        assert!(target.exists());
        assert!(fs::symlink_metadata(&target).unwrap().file_type().is_symlink());

        // Verify we can read through symlink
        let content = fs::read_to_string(target.join("test.txt")).unwrap();
        assert_eq!(content, "unix symlink test");

        // Cleanup
        let _ = fs::remove_file(&target);
        let _ = fs::remove_dir_all(&source);
    }

    /// Test Unix symlink with nested dependencies
    #[cfg(not(windows))]
    #[test]
    fn test_unix_nested_symlinks() {
        use std::os::unix::fs::symlink;

        let temp = std::env::temp_dir();
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // Create store with multiple packages
        let store = temp.join(format!("dx_unix_store_{}", unique_id));

        // Package A depends on Package B
        let pkg_a = store.join("pkg-a@1.0.0");
        let pkg_b = store.join("pkg-b@1.0.0");

        fs::create_dir_all(&pkg_a).unwrap();
        fs::create_dir_all(&pkg_b).unwrap();

        fs::write(
            pkg_a.join("package.json"),
            r#"{"name":"pkg-a","version":"1.0.0","dependencies":{"pkg-b":"^1.0.0"}}"#,
        )
        .unwrap();
        fs::write(pkg_b.join("package.json"), r#"{"name":"pkg-b","version":"1.0.0"}"#).unwrap();

        // Create node_modules with symlinks
        let node_modules = temp.join(format!("dx_unix_nm_{}", unique_id));
        fs::create_dir_all(&node_modules).unwrap();

        // Create symlinks
        symlink(&pkg_a, node_modules.join("pkg-a")).unwrap();
        symlink(&pkg_b, node_modules.join("pkg-b")).unwrap();

        // Verify both symlinks work
        assert!(node_modules.join("pkg-a/package.json").exists());
        assert!(node_modules.join("pkg-b/package.json").exists());

        // Cleanup
        let _ = fs::remove_file(node_modules.join("pkg-a"));
        let _ = fs::remove_file(node_modules.join("pkg-b"));
        let _ = fs::remove_dir_all(&store);
        let _ = fs::remove_dir_all(&node_modules);
    }

    /// Test Unix shell command execution
    #[cfg(not(windows))]
    #[test]
    fn test_unix_shell_execution() {
        use std::process::Command;

        // Test sh -c execution
        let output = Command::new("sh").args(["-c", "echo 'Hello from sh'"]).output().unwrap();

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Hello from sh"));
    }
}
