use indicatif::{ProgressBar, ProgressStyle};

pub fn create_progress_bar(total: u64) -> ProgressBar {
    let bar = ProgressBar::new(total);
    let style = ProgressStyle::with_template(
        "{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})",
    )
    .unwrap_or_else(|_| ProgressStyle::default_bar())
    .progress_chars("#>-");
    bar.set_style(style);
    bar
}

pub fn create_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(std::time::Duration::from_millis(120));
    spinner
}
