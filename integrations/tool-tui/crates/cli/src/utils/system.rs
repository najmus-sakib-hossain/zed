//! System utilities

/// Check if a command exists on the system
#[allow(dead_code)]
pub fn command_exists(cmd: &str) -> bool {
    which::which(cmd).is_ok()
}

/// Get available memory in bytes
#[allow(dead_code)]
pub fn available_memory() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/meminfo").ok().and_then(|content| {
            content.lines().find(|line| line.starts_with("MemAvailable:")).and_then(|line| {
                line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u64>().ok())
                    .map(|kb| kb * 1024)
            })
        })
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|output| {
                String::from_utf8(output.stdout).ok().and_then(|s| s.trim().parse().ok())
            })
    }

    #[cfg(target_os = "windows")]
    {
        // Would need Windows API for full implementation
        None
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

/// Get the number of available CPU cores
#[allow(dead_code)]
pub fn cpu_count() -> usize {
    std::thread::available_parallelism().map(|p| p.get()).unwrap_or(1)
}

/// Check if running in a CI environment
#[allow(dead_code)]
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
}
