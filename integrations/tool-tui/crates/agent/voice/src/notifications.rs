//! Cross-platform notification support

use anyhow::Result;

/// Send a system notification
pub fn notify(title: &str, body: &str) -> Result<()> {
    notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show()?;
    Ok(())
}

/// Send a notification with an icon
pub fn notify_with_icon(title: &str, body: &str, icon: &str) -> Result<()> {
    notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .icon(icon)
        .timeout(notify_rust::Timeout::Milliseconds(5000))
        .show()?;
    Ok(())
}

/// Send an urgent notification
pub fn notify_urgent(title: &str, body: &str) -> Result<()> {
    notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .timeout(notify_rust::Timeout::Never)
        .show()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_notification_module_compiles() {
        // Notification sending requires a desktop environment
        // Just verify the module compiles
        assert!(true);
    }
}
