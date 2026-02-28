//! Process object implementation for Node.js compatibility.
//!
//! This module provides the `process` global object functionality.

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Get the current platform name (like Node.js process.platform).
pub fn platform() -> &'static str {
    #[cfg(target_os = "windows")]
    return "win32";
    #[cfg(target_os = "macos")]
    return "darwin";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(target_os = "freebsd")]
    return "freebsd";
    #[cfg(target_os = "openbsd")]
    return "openbsd";
    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd"
    )))]
    return "unknown";
}

/// Get the CPU architecture (like Node.js process.arch).
pub fn arch() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    return "x64";
    #[cfg(target_arch = "x86")]
    return "ia32";
    #[cfg(target_arch = "aarch64")]
    return "arm64";
    #[cfg(target_arch = "arm")]
    return "arm";
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "x86",
        target_arch = "aarch64",
        target_arch = "arm"
    )))]
    return "unknown";
}

/// Get the current working directory.
pub fn cwd() -> PathBuf {
    env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Change the current working directory.
pub fn chdir(dir: &str) -> Result<(), std::io::Error> {
    env::set_current_dir(dir)
}

/// Get environment variables as a HashMap.
pub fn env_vars() -> HashMap<String, String> {
    env::vars().collect()
}

/// Get a specific environment variable.
pub fn env_get(key: &str) -> Option<String> {
    env::var(key).ok()
}

/// Set an environment variable.
pub fn env_set(key: &str, value: &str) {
    env::set_var(key, value);
}

/// Remove an environment variable.
pub fn env_remove(key: &str) {
    env::remove_var(key);
}

/// Get command line arguments (like Node.js process.argv).
pub fn argv() -> Vec<String> {
    env::args().collect()
}

/// Get the process ID.
pub fn pid() -> u32 {
    std::process::id()
}

/// Get the parent process ID.
#[cfg(unix)]
pub fn ppid() -> u32 {
    unsafe { libc::getppid() as u32 }
}

/// Get the parent process ID (Windows stub).
#[cfg(windows)]
pub fn ppid() -> u32 {
    0 // Windows doesn't have a simple way to get ppid
}

/// Exit the process with the given code.
pub fn exit(code: i32) -> ! {
    std::process::exit(code)
}

/// Get the executable path.
pub fn exec_path() -> PathBuf {
    env::current_exe().unwrap_or_else(|_| PathBuf::from(""))
}

/// Get Node.js version (returns dx-js version).
pub fn version() -> &'static str {
    "v20.0.0" // Compatibility version
}

/// Get versions object.
pub fn versions() -> HashMap<&'static str, &'static str> {
    let mut versions = HashMap::new();
    versions.insert("node", "20.0.0");
    versions.insert("dx", env!("CARGO_PKG_VERSION"));
    versions.insert("v8", "11.0.0"); // Compatibility
    versions
}

/// Memory usage information.
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// Resident set size in bytes
    pub rss: u64,
    /// Total heap size in bytes
    pub heap_total: u64,
    /// Used heap size in bytes
    pub heap_used: u64,
    /// External memory in bytes
    pub external: u64,
    /// Array buffers in bytes
    pub array_buffers: u64,
}

/// Get memory usage (approximate).
pub fn memory_usage() -> MemoryUsage {
    // This is a simplified implementation
    // Real implementation would use platform-specific APIs
    MemoryUsage {
        rss: 0,
        heap_total: 0,
        heap_used: 0,
        external: 0,
        array_buffers: 0,
    }
}

/// CPU usage information.
#[derive(Debug, Clone)]
pub struct CpuUsage {
    /// User CPU time in microseconds
    pub user: u64,
    /// System CPU time in microseconds
    pub system: u64,
}

/// Get CPU usage.
pub fn cpu_usage() -> CpuUsage {
    CpuUsage { user: 0, system: 0 }
}

/// High-resolution time in nanoseconds.
pub fn hrtime() -> (u64, u64) {
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    let elapsed = start.elapsed();
    (elapsed.as_secs(), elapsed.subsec_nanos() as u64)
}

/// High-resolution time as BigInt (nanoseconds).
pub fn hrtime_bigint() -> u128 {
    let (secs, nanos) = hrtime();
    (secs as u128) * 1_000_000_000 + (nanos as u128)
}

/// Get uptime in seconds.
pub fn uptime() -> f64 {
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    let start = START.get_or_init(Instant::now);
    start.elapsed().as_secs_f64()
}

/// Schedule a callback to run on next tick.
/// Note: This is a simplified implementation - real implementation
/// would integrate with the event loop.
pub fn next_tick<F: FnOnce() + Send + 'static>(callback: F) {
    // In a real implementation, this would queue to the event loop
    // For now, we just spawn a task
    std::thread::spawn(callback);
}

/// Standard streams
pub mod stdio {
    use std::io::{self, Read, Write};

    /// Write to stdout.
    pub fn stdout_write(data: &[u8]) -> io::Result<usize> {
        io::stdout().write(data)
    }

    /// Write to stderr.
    pub fn stderr_write(data: &[u8]) -> io::Result<usize> {
        io::stderr().write(data)
    }

    /// Read from stdin.
    pub fn stdin_read(buf: &mut [u8]) -> io::Result<usize> {
        io::stdin().read(buf)
    }

    /// Check if stdout is a TTY.
    #[cfg(unix)]
    pub fn stdout_is_tty() -> bool {
        unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
    }

    /// Check if stdout is a TTY (Windows).
    #[cfg(windows)]
    pub fn stdout_is_tty() -> bool {
        use std::os::windows::io::AsRawHandle;
        let handle = io::stdout().as_raw_handle();
        is_console_handle(handle)
    }

    /// Check if stderr is a TTY.
    #[cfg(unix)]
    pub fn stderr_is_tty() -> bool {
        unsafe { libc::isatty(libc::STDERR_FILENO) != 0 }
    }

    /// Check if stderr is a TTY (Windows).
    #[cfg(windows)]
    pub fn stderr_is_tty() -> bool {
        use std::os::windows::io::AsRawHandle;
        let handle = io::stderr().as_raw_handle();
        is_console_handle(handle)
    }

    /// Check if stdin is a TTY.
    #[cfg(unix)]
    pub fn stdin_is_tty() -> bool {
        unsafe { libc::isatty(libc::STDIN_FILENO) != 0 }
    }

    /// Check if stdin is a TTY (Windows).
    #[cfg(windows)]
    pub fn stdin_is_tty() -> bool {
        use std::os::windows::io::AsRawHandle;
        let handle = io::stdin().as_raw_handle();
        is_console_handle(handle)
    }

    /// Helper to check if a handle is a console on Windows.
    #[cfg(windows)]
    fn is_console_handle(handle: std::os::windows::io::RawHandle) -> bool {
        // Use GetFileType to check if it's a character device (console)
        extern "system" {
            fn GetFileType(hFile: *mut std::ffi::c_void) -> u32;
        }
        const FILE_TYPE_CHAR: u32 = 0x0002;
        unsafe { GetFileType(handle as *mut _) == FILE_TYPE_CHAR }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform() {
        let p = platform();
        assert!(!p.is_empty());
        #[cfg(target_os = "windows")]
        assert_eq!(p, "win32");
        #[cfg(target_os = "linux")]
        assert_eq!(p, "linux");
        #[cfg(target_os = "macos")]
        assert_eq!(p, "darwin");
    }

    #[test]
    fn test_arch() {
        let a = arch();
        assert!(!a.is_empty());
        #[cfg(target_arch = "x86_64")]
        assert_eq!(a, "x64");
        #[cfg(target_arch = "aarch64")]
        assert_eq!(a, "arm64");
    }

    #[test]
    fn test_cwd() {
        let cwd = cwd();
        assert!(cwd.exists());
    }

    #[test]
    fn test_env() {
        env_set("DX_TEST_VAR", "test_value");
        assert_eq!(env_get("DX_TEST_VAR"), Some("test_value".to_string()));
        env_remove("DX_TEST_VAR");
        assert_eq!(env_get("DX_TEST_VAR"), None);
    }

    #[test]
    fn test_argv() {
        let args = argv();
        assert!(!args.is_empty());
    }

    #[test]
    fn test_pid() {
        let p = pid();
        assert!(p > 0);
    }

    #[test]
    fn test_version() {
        let v = version();
        assert!(v.starts_with('v'));
    }

    #[test]
    fn test_hrtime() {
        let (secs1, nanos1) = hrtime();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let (secs2, nanos2) = hrtime();

        let time1 = secs1 * 1_000_000_000 + nanos1;
        let time2 = secs2 * 1_000_000_000 + nanos2;
        assert!(time2 > time1);
    }

    #[test]
    fn test_uptime() {
        let u1 = uptime();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let u2 = uptime();
        assert!(u2 > u1);
    }
}
