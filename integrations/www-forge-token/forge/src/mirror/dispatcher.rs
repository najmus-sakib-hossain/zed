use super::{MediaType, MirrorBackend, MirrorError, MirrorMetadata, MirrorTarget};
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::Arc;

pub struct MirrorResult {
    pub backend: &'static str,
    pub target: Result<MirrorTarget, MirrorError>,
}

pub struct MirrorDispatcher {
    backends: Vec<Arc<dyn MirrorBackend>>,
}

impl MirrorDispatcher {
    pub fn new(backends: Vec<Arc<dyn MirrorBackend>>) -> Self {
        Self { backends }
    }

    /// Mirror `data` from `path` to all capable backends in parallel.
    pub async fn mirror(&self, path: &Path, data: Vec<u8>) -> Vec<MirrorResult> {
        let media_type = MediaType::from_path(path);
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let meta = Arc::new(MirrorMetadata {
            filename: filename.clone(),
            media_type: media_type.clone(),
            description: None,
        });

        let capable: Vec<Arc<dyn MirrorBackend>> = self
            .backends
            .iter()
            .filter(|b| b.can_handle(&media_type))
            .cloned()
            .collect();

        if capable.is_empty() {
            tracing::debug!("no mirror backend capable of handling {:?}", media_type);
            return vec![];
        }

        let pb = ProgressBar::new(capable.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("  mirroring {msg} [{bar:40.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("=>-"),
        );
        pb.set_message(filename.clone());

        let tasks: Vec<_> = capable
            .iter()
            .map(|b| {
                let b = Arc::clone(b);
                let meta = Arc::clone(&meta);
                let data = data.clone();
                let pb = pb.clone();
                async move {
                    let name = b.name();
                    let target = b.upload(data, &meta).await;
                    pb.inc(1);
                    MirrorResult { backend: name, target }
                }
            })
            .collect();

        let results = join_all(tasks).await;
        pb.finish_with_message(format!("✓ {filename}"));
        results
    }
}
