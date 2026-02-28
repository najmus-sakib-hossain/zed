//! Background conversion daemon
//! Converts tarballs to binary format WITHOUT blocking install

use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Clone)]
pub struct ConversionJob {
    pub name: String,
    pub version: String,
    pub tarball_path: PathBuf,
    pub priority: Priority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum Priority {
    Low = 0,    // Convert when idle
    Normal = 1, // Convert after install completes
    High = 2,   // Convert ASAP (user explicitly requested)
}

/// Background converter daemon
#[allow(dead_code)]
pub struct BackgroundConverter {
    tx: mpsc::UnboundedSender<ConversionJob>,
}

impl BackgroundConverter {
    /// Start the background converter daemon
    pub fn new(binary_dir: PathBuf) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // Spawn the converter task
        tokio::spawn(async move {
            Self::run_daemon(rx, binary_dir).await;
        });

        Self { tx }
    }

    /// Queue a package for background conversion
    #[allow(dead_code)]
    pub fn queue(&self, job: ConversionJob) -> Result<()> {
        self.tx.send(job)?;
        Ok(())
    }

    /// Queue multiple packages at once
    #[allow(dead_code)]
    pub fn queue_many(&self, jobs: Vec<ConversionJob>) -> Result<()> {
        for job in jobs {
            self.tx.send(job)?;
        }
        Ok(())
    }

    /// Background daemon loop
    async fn run_daemon(mut rx: mpsc::UnboundedReceiver<ConversionJob>, binary_dir: PathBuf) {
        let converter = dx_pkg_converter::PackageConverter::new();
        let mut in_progress: HashSet<String> = HashSet::new();
        let mut queue: Vec<ConversionJob> = Vec::new();

        // Create binary cache directory
        std::fs::create_dir_all(&binary_dir).ok();

        loop {
            tokio::select! {
                // Receive new jobs
                Some(job) = rx.recv() => {
                    let key = format!("{}@{}", job.name, job.version);

                    // Skip if already in progress or already exists
                    let binary_path = binary_dir.join(format!("{}-{}.dxp",
                        job.name.replace('/', "-"), job.version));

                    if !in_progress.contains(&key) && !binary_path.exists() {
                        queue.push(job);
                        // Sort by priority (high to low)
                        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
                    }
                }

                // Process queue when we have jobs
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)), if !queue.is_empty() => {
                    if let Some(job) = queue.pop() {
                        let key = format!("{}@{}", job.name, job.version);
                        in_progress.insert(key.clone());

                        // Convert in blocking thread (CPU-bound work)
                        let binary_dir = binary_dir.clone();
                        let converter = converter.clone();

                        tokio::task::spawn_blocking(move || {
                            Self::convert_package(&job, &binary_dir, &converter)
                        }).await.ok();

                        in_progress.remove(&key);
                    }
                }

                // Keep daemon alive
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)), if queue.is_empty() => {
                    // Idle - daemon stays alive
                }
            }
        }
    }

    /// Convert a single package
    fn convert_package(
        job: &ConversionJob,
        binary_dir: &Path,
        converter: &dx_pkg_converter::PackageConverter,
    ) -> Result<()> {
        let start = Instant::now();

        // Read tarball
        let tgz_data = std::fs::read(&job.tarball_path)?;

        // Convert using async runtime
        let runtime = tokio::runtime::Handle::current();
        let _binary_path = runtime.block_on(async {
            converter.convert_bytes(&job.name, &job.version, &tgz_data, binary_dir).await
        })?;

        let elapsed = start.elapsed();
        tracing::debug!(
            "✓ Converted {}@{} to binary cache in {:.2}ms",
            job.name,
            job.version,
            elapsed.as_secs_f64() * 1000.0
        );

        Ok(())
    }
}

/// Global background converter instance
static CONVERTER: once_cell::sync::Lazy<Arc<Mutex<Option<BackgroundConverter>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// Initialize the global background converter
pub async fn init_background_converter(binary_dir: PathBuf) {
    let mut converter = CONVERTER.lock().await;
    if converter.is_none() {
        *converter = Some(BackgroundConverter::new(binary_dir));
    }
}

/// Queue a job with the global converter
#[allow(dead_code)]
pub async fn queue_conversion(job: ConversionJob) -> Result<()> {
    let converter = CONVERTER.lock().await;
    if let Some(conv) = converter.as_ref() {
        conv.queue(job)?;
    }
    Ok(())
}

/// Queue multiple jobs
#[allow(dead_code)]
pub async fn queue_conversions(jobs: Vec<ConversionJob>) -> Result<()> {
    let converter = CONVERTER.lock().await;
    if let Some(conv) = converter.as_ref() {
        conv.queue_many(jobs)?;
    }
    Ok(())
}
