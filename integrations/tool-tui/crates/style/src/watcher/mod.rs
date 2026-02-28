//! File Watcher module
//!
//! Provides file system watching for automatic CSS regeneration on HTML changes.
//! Supports debounced events, polling mode, and raw event mode for different
//! performance characteristics.

use crate::config::RebuildConfig;
use crate::core::{AppState, rebuild_styles};
use colored::Colorize;
use notify::RecursiveMode;
use notify::{Event, Watcher};
use notify_debouncer_full::new_debouncer;
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use crate::config::Config;

pub fn start(
    state: Arc<Mutex<AppState>>,
    config: Config,
    rebuild_config: RebuildConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(poll_ms_str) = std::env::var("DX_WATCH_POLL_MS") {
        if let Ok(interval_ms) = poll_ms_str.parse::<u64>() {
            let interval = Duration::from_millis(interval_ms.max(1));
            use std::fs;
            let mut last_mtime =
                fs::metadata(&config.paths.index_file).and_then(|m| m.modified()).ok();
            loop {
                std::thread::sleep(interval);
                if let Ok(meta) = fs::metadata(&config.paths.index_file) {
                    if let Ok(modified) = meta.modified() {
                        if last_mtime.map(|t| t != modified).unwrap_or(true) {
                            last_mtime = Some(modified);
                            if let Err(e) = rebuild_styles(
                                state.clone(),
                                &config.paths.index_file,
                                false,
                                &rebuild_config,
                            ) {
                                eprintln!("{} {}", "Error rebuilding styles:".red(), e);
                            }
                        }
                    }
                }
            }
        }
    }

    if std::env::var("DX_WATCH_RAW").ok().as_deref() == Some("1") {
        let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;
        watcher.watch(Path::new(&config.paths.html_dir), RecursiveMode::Recursive)?;
        let mut last_trigger = Instant::now() - Duration::from_secs(1);
        let min_gap = Duration::from_millis(5);
        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    let relevant = event.paths.iter().any(|p| {
                        if let Some(s) = p.to_str() {
                            s.ends_with("index.html")
                        } else {
                            false
                        }
                    });
                    if relevant && last_trigger.elapsed() >= min_gap {
                        last_trigger = Instant::now();
                        if let Err(e) = rebuild_styles(
                            state.clone(),
                            &config.paths.index_file,
                            false,
                            &rebuild_config,
                        ) {
                            eprintln!("{} {}", "Error rebuilding styles:".red(), e);
                        }
                    }
                }
                Ok(Err(e)) => eprintln!("{} {:?}", "Watch error:".red(), e),
                Err(_) => break,
            }
        }
        return Ok(());
    }

    let (tx, rx) = mpsc::channel();
    let debounce_ms = std::env::var("DX_DEBOUNCE_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .or_else(|| config.watch.as_ref().and_then(|w| w.debounce_ms))
        .unwrap_or(250);

    let mut debouncer = new_debouncer(Duration::from_millis(debounce_ms.max(1)), None, tx)?;
    debouncer.watch(Path::new(&config.paths.html_dir), RecursiveMode::Recursive)?;

    loop {
        let res = rx.recv();
        match res {
            Ok(Ok(events)) => {
                let mut relevant = false;
                for ev in events {
                    for path in &ev.paths {
                        if let Some(p) = path.to_str() {
                            if p.ends_with("index.html") || p.ends_with("style.css") {
                                relevant = true;
                                break;
                            }
                        }
                    }
                    if relevant {
                        break;
                    }
                }
                if relevant {
                    if let Err(e) = rebuild_styles(
                        state.clone(),
                        &config.paths.index_file,
                        false,
                        &rebuild_config,
                    ) {
                        eprintln!("{} {}", "Error rebuilding styles:".red(), e);
                    }
                }
            }
            Ok(Err(e)) => eprintln!("{} {:?}", "Watch error:".red(), e),
            Err(_) => break,
        }
    }

    Ok(())
}
