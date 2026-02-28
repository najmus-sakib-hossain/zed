//! GitHub-like Web UI for Forge
//!
//! Features:
//! - Browse files and folders (like GitHub)
//! - View file contents with syntax highlighting
//! - Download individual files
//! - Download entire repository as ZIP
//! - Responsive design with Tailwind CSS

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Path as AxumPath, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;
use tower_http::services::ServeDir;
use zip::write::{FileOptions, ZipWriter};

// Import dx_forge modules
use dx_forge::storage::r2::{R2Config, R2Storage};

/// File tree node
#[derive(Debug, Clone, Serialize)]
struct FileNode {
    name: String,
    path: String,
    #[serde(rename = "type")]
    node_type: String, // "file" or "directory"
    size: Option<u64>,
    hash: Option<String>,
    children: Option<Vec<FileNode>>,
}

/// File content response
#[derive(Debug, Serialize)]
struct FileContent {
    path: String,
    content: String,
    size: u64,
    hash: String,
    language: String,
}

/// Application state
#[derive(Clone)]
struct AppState {
    #[allow(dead_code)]
    r2_storage: Arc<R2Storage>,
    demo_root: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load R2 configuration
    let r2_config = R2Config::from_env()?;
    let r2_storage = Arc::new(R2Storage::new(r2_config)?);

    let demo_root = "examples/forge-demo".to_string();

    // Initialize Forge storage if not exists
    let demo_path = Path::new(&demo_root);
    let forge_path = demo_path.join(".dx/forge");
    let needs_init = !forge_path.exists();

    if needs_init {
        println!("🔨 Initializing Forge storage at {}", forge_path.display());
        dx_forge::storage::init(demo_path).await?;
        println!("✅ Forge storage initialized");

        // Initialize repository with existing files
        println!("📝 Scanning and storing existing files...");
        initialize_repository(demo_path).await?;
        println!("✅ Repository initialized with {} files", count_files(demo_path).await?);
    } else {
        println!("📦 Using existing Forge storage at {}", forge_path.display());
    }

    let state = AppState {
        r2_storage,
        demo_root,
    };

    // Build router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/tree", get(get_file_tree))
        .route("/api/file/{*path}", get(get_file_content))
        .route("/api/download/{*path}", get(download_file))
        .route("/api/download-zip", post(download_as_zip))
        .nest_service("/static", ServeDir::new("examples/web-ui/static"))
        .with_state(state);

    // Start server
    let addr = "127.0.0.1:3000";
    println!("🚀 Forge Web UI running at http://{}", addr);
    println!("📁 Serving: examples/forge-demo");
    println!("🌐 Open your browser to explore files!");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Index page handler
async fn index_handler() -> Html<String> {
    Html(HTML_TEMPLATE.to_string())
}

/// Get file tree
async fn get_file_tree(State(state): State<AppState>) -> Result<Json<FileNode>, StatusCode> {
    let root = build_file_tree(&state.demo_root, &state.demo_root)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(root))
}

/// Build file tree recursively
fn build_file_tree<'a>(
    root: &'a str,
    path: &'a str,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<FileNode>> + Send + 'a>> {
    Box::pin(async move {
        let metadata = fs::metadata(path).await?;
        let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy().to_string();

        let relative_path = path.strip_prefix(root).unwrap_or(path);
        // Normalize path separators for URLs (Windows uses backslashes)
        let relative_path = relative_path.replace('\\', "/");
        // Remove any leading slashes to ensure clean paths
        let relative_path = relative_path.trim_start_matches('/');

        if metadata.is_dir() {
            let mut children = Vec::new();
            let mut entries = fs::read_dir(path).await?;

            while let Some(entry) = entries.next_entry().await? {
                let child_path = entry.path().to_string_lossy().to_string();

                // Skip hidden files except .forge
                let child_name = entry.file_name().to_string_lossy().to_string();
                if child_name.starts_with('.') && child_name != ".forge" {
                    continue;
                }

                if let Ok(child) = build_file_tree(root, &child_path).await {
                    children.push(child);
                }
            }

            // Sort: directories first, then alphabetically
            children.sort_by(|a, b| match (a.node_type.as_str(), b.node_type.as_str()) {
                ("directory", "file") => std::cmp::Ordering::Less,
                ("file", "directory") => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });

            Ok(FileNode {
                name,
                path: relative_path.to_string(),
                node_type: "directory".to_string(),
                size: None,
                hash: None,
                children: Some(children),
            })
        } else {
            // Calculate SHA-256 hash of file content
            let content = fs::read(path).await?;
            let hash = {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(&content);
                format!("{:x}", hasher.finalize())
            };

            Ok(FileNode {
                name,
                path: relative_path.to_string(),
                node_type: "file".to_string(),
                size: Some(metadata.len()),
                hash: Some(hash),
                children: None,
            })
        }
    })
}

/// Get file content
async fn get_file_content(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Result<Json<FileContent>, StatusCode> {
    // Clean up path: remove leading slashes and normalize separators
    let clean_path = path.trim_start_matches('/').replace('\\', "/");
    let full_path = format!("{}/{}", state.demo_root, clean_path);

    let content = fs::read_to_string(&full_path).await.map_err(|e| {
        eprintln!("Failed to read file '{}': {}", full_path, e);
        StatusCode::NOT_FOUND
    })?;

    let metadata = fs::metadata(&full_path).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Detect language from extension
    let language = detect_language(&clean_path);

    // Calculate SHA-256 hash for content (using sha2 for consistency with rest of codebase)
    let hash = {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    Ok(Json(FileContent {
        path: clean_path,
        content,
        size: metadata.len(),
        hash,
        language,
    }))
}

/// Download individual file
async fn download_file(
    State(state): State<AppState>,
    AxumPath(path): AxumPath<String>,
) -> Result<Response, StatusCode> {
    // Clean up path: remove leading slashes and normalize separators
    let clean_path = path.trim_start_matches('/').replace('\\', "/");
    let full_path = format!("{}/{}", state.demo_root, clean_path);

    let content = fs::read(&full_path).await.map_err(|e| {
        eprintln!("Failed to read file '{}': {}", full_path, e);
        StatusCode::NOT_FOUND
    })?;

    let filename = std::path::Path::new(&path).file_name().unwrap().to_string_lossy().to_string();

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CONTENT_DISPOSITION, &format!("attachment; filename=\"{}\"", filename)),
        ],
        content,
    )
        .into_response())
}

/// Download repository as ZIP
async fn download_as_zip(State(state): State<AppState>) -> Result<Response, StatusCode> {
    use std::io::Cursor;

    let mut zip_buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut zip_buffer);

    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Add all files to ZIP
    add_directory_to_zip(&mut zip, &state.demo_root, &state.demo_root, options)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    zip.finish().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let zip_data = zip_buffer.into_inner();

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/zip"),
            (header::CONTENT_DISPOSITION, "attachment; filename=\"forge-demo.zip\""),
        ],
        zip_data,
    )
        .into_response())
}

/// Recursively add directory to ZIP
fn add_directory_to_zip<'a>(
    zip: &'a mut ZipWriter<&mut Cursor<Vec<u8>>>,
    root: &'a str,
    path: &'a str,
    options: FileOptions<'static, ()>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let entry_path = entry.path();
            let entry_path_str = entry_path.to_string_lossy();

            // Skip hidden files except .forge
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') && name != ".forge" {
                continue;
            }

            let relative_path = entry_path_str.strip_prefix(root).unwrap_or(&entry_path_str);
            let relative_path = relative_path.trim_start_matches('/').trim_start_matches('\\');

            if entry.file_type().await?.is_dir() {
                // Add directory
                zip.add_directory(relative_path, options)?;

                // Recursively add contents
                add_directory_to_zip(zip, root, &entry_path_str, options).await?;
            } else {
                // Add file
                let content = fs::read(&entry_path).await?;
                zip.start_file(relative_path, options)?;
                std::io::Write::write_all(zip, &content)?;
            }
        }

        Ok(())
    })
}

/// Detect programming language from file extension
fn detect_language(path: &str) -> String {
    let ext = std::path::Path::new(path).extension().and_then(|s| s.to_str()).unwrap_or("");

    match ext {
        "rs" => "rust",
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "java" => "java",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "go" => "go",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" => "kotlin",
        "sh" | "bash" => "bash",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "html" => "html",
        "css" => "css",
        "md" => "markdown",
        "sql" => "sql",
        _ => "plaintext",
    }
    .to_string()
}

/// HTML template (GitHub-like UI)
const HTML_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Forge Demo - File Browser</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github-dark.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
    <style>
        .file-tree-item:hover { background-color: #2d333b; cursor: pointer; }
        .file-tree-item.active { background-color: #1f6feb; }
        .folder-icon::before { content: "📁 "; }
        .file-icon::before { content: "📄 "; }
        pre { margin: 0; }
        code { font-family: 'Consolas', 'Monaco', monospace; font-size: 14px; }
    </style>
</head>
<body class="bg-gray-900 text-gray-100">
    <!-- Header -->
    <header class="bg-gray-800 border-b border-gray-700 p-4">
        <div class="container mx-auto flex justify-between items-center">
            <h1 class="text-2xl font-bold">🔥 Forge Demo</h1>
            <div class="space-x-4">
                <button onclick="downloadZip()" class="bg-green-600 hover:bg-green-700 px-4 py-2 rounded">
                    🗜️ Download ZIP
                </button>
                <a href="https://github.com" class="text-blue-400 hover:underline">GitHub</a>
            </div>
        </div>
    </header>

    <!-- Main Content -->
    <div class="container mx-auto mt-6 flex gap-4">
        <!-- File Tree (Left Sidebar) -->
        <aside class="w-1/4 bg-gray-800 rounded-lg p-4">
            <h2 class="text-lg font-semibold mb-4 border-b border-gray-700 pb-2">📂 Files</h2>
            <div id="file-tree" class="space-y-1"></div>
        </aside>

        <!-- File Viewer (Main Content) -->
        <main class="flex-1 bg-gray-800 rounded-lg p-6">
            <div id="file-header" class="border-b border-gray-700 pb-4 mb-4 hidden">
                <div class="flex justify-between items-center">
                    <div>
                        <h2 id="file-name" class="text-xl font-semibold"></h2>
                        <p id="file-meta" class="text-sm text-gray-400 mt-1"></p>
                    </div>
                    <button onclick="downloadCurrentFile()" class="bg-blue-600 hover:bg-blue-700 px-4 py-2 rounded">
                        📥 Download
                    </button>
                </div>
            </div>
            
            <div id="file-content">
                <div class="text-center text-gray-400 py-20">
                    <p class="text-6xl mb-4">🔥</p>
                    <p class="text-xl">Select a file to view its contents</p>
                    <p class="text-sm mt-2">Or download the entire repository as ZIP</p>
                </div>
            </div>
        </main>
    </div>

    <!-- Footer -->
    <footer class="container mx-auto mt-8 text-center text-gray-400 pb-6">
        <p>Built with ❤️ and Rust 🦀 | <a href="/api/tree" class="text-blue-400 hover:underline">API</a></p>
    </footer>

    <script>
        let currentFile = null;
        let fileTree = null;

        // Load file tree on page load
        window.addEventListener('DOMContentLoaded', async () => {
            await loadFileTree();
        });

        // Load file tree from API
        async function loadFileTree() {
            try {
                const response = await fetch('/api/tree');
                fileTree = await response.json();
                renderFileTree(fileTree, document.getElementById('file-tree'), 0);
            } catch (error) {
                console.error('Failed to load file tree:', error);
                document.getElementById('file-tree').innerHTML = '<p class="text-red-400">Failed to load files</p>';
            }
        }

        // Render file tree recursively
        function renderFileTree(node, container, depth) {
            if (node.type === 'directory' && node.children) {
                // Directory
                const dirDiv = document.createElement('div');
                dirDiv.className = 'ml-' + (depth * 4);
                
                const dirHeader = document.createElement('div');
                dirHeader.className = 'file-tree-item folder-icon px-2 py-1 rounded flex items-center';
                dirHeader.textContent = node.name;
                dirHeader.onclick = () => {
                    const childrenDiv = dirDiv.querySelector('.children');
                    childrenDiv.classList.toggle('hidden');
                };
                
                dirDiv.appendChild(dirHeader);
                
                const childrenDiv = document.createElement('div');
                childrenDiv.className = 'children ml-4';
                node.children.forEach(child => renderFileTree(child, childrenDiv, depth + 1));
                dirDiv.appendChild(childrenDiv);
                
                container.appendChild(dirDiv);
            } else {
                // File
                const fileDiv = document.createElement('div');
                fileDiv.className = 'file-tree-item file-icon px-2 py-1 rounded ml-' + (depth * 4);
                fileDiv.textContent = node.name;
                fileDiv.onclick = (e) => loadFile(node.path, e.currentTarget);
                container.appendChild(fileDiv);
            }
        }

        // Load and display file content
        async function loadFile(path, clickedElement) {
            try {
                const response = await fetch(`/api/file/${path}`);
                const data = await response.json();
                currentFile = data;
                
                // Update header
                document.getElementById('file-header').classList.remove('hidden');
                document.getElementById('file-name').textContent = path;
                document.getElementById('file-meta').textContent = 
                    `${formatBytes(data.size)} • ${data.language} • ${data.hash.substring(0, 8)}`;
                
                // Render content with syntax highlighting
                const contentDiv = document.getElementById('file-content');
                contentDiv.innerHTML = `<pre><code class="language-${data.language}">${escapeHtml(data.content)}</code></pre>`;
                hljs.highlightAll();
                
                // Highlight active file in tree
                document.querySelectorAll('.file-tree-item').forEach(el => el.classList.remove('active'));
                if (clickedElement) {
                    clickedElement.classList.add('active');
                }
            } catch (error) {
                console.error('Failed to load file:', error);
                document.getElementById('file-content').innerHTML = 
                    '<p class="text-red-400">Failed to load file</p>';
            }
        }

        // Download current file
        function downloadCurrentFile() {
            if (currentFile) {
                window.location.href = `/api/download/${currentFile.path}`;
            }
        }

        // Download entire repository as ZIP
        async function downloadZip() {
            try {
                const response = await fetch('/api/download-zip', { method: 'POST' });
                const blob = await response.blob();
                const url = window.URL.createObjectURL(blob);
                const a = document.createElement('a');
                a.href = url;
                a.download = 'forge-demo.zip';
                document.body.appendChild(a);
                a.click();
                window.URL.revokeObjectURL(url);
                document.body.removeChild(a);
            } catch (error) {
                console.error('Failed to download ZIP:', error);
                alert('Failed to download ZIP');
            }
        }

        // Utility: Format bytes
        function formatBytes(bytes) {
            if (bytes === 0) return '0 Bytes';
            const k = 1024;
            const sizes = ['Bytes', 'KB', 'MB', 'GB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
        }

        // Utility: Escape HTML
        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
    </script>
</body>
</html>
"#;

/// Initialize repository by storing all existing files as blobs
async fn initialize_repository(repo_root: &Path) -> Result<()> {
    use dx_forge::storage::Database;

    let forge_path = repo_root.join(".dx/forge");
    let db = Database::new(&forge_path)?;
    db.initialize()?;

    // Initialize refs (like Git refs)
    initialize_refs(&forge_path).await?;

    // Initialize logs
    initialize_logs(&forge_path).await?;

    // Initialize context
    initialize_context(&forge_path).await?;

    // Recursively scan and store files
    store_directory_blobs(repo_root, repo_root, &forge_path).await?;

    Ok(())
}

/// Recursively store files as blobs
fn store_directory_blobs<'a>(
    repo_root: &'a Path,
    dir_path: &'a Path,
    forge_path: &'a Path,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip .dx and .git directories
            if name.starts_with('.') && (name == ".dx" || name == ".git" || name == ".forge") {
                continue;
            }

            if entry.file_type().await?.is_dir() {
                // Recursively process directory
                store_directory_blobs(repo_root, &path, forge_path).await?;
            } else {
                // Store file as blob
                let content = fs::read(&path).await?;
                let hash = {
                    let mut hasher = Sha256::new();
                    hasher.update(&content);
                    format!("{:x}", hasher.finalize())
                };

                // Store in objects directory (content-addressable)
                let hash_dir = forge_path.join("objects").join(&hash[..2]);
                tokio::fs::create_dir_all(&hash_dir).await?;
                let blob_path = hash_dir.join(&hash[2..]);

                if !blob_path.exists() {
                    tokio::fs::write(&blob_path, &content).await?;

                    let relative_path =
                        path.strip_prefix(repo_root).unwrap_or(&path).display().to_string();

                    println!("  📄 Stored: {} ({})", relative_path, &hash[..8]);
                }
            }
        }

        Ok(())
    })
}

/// Count files in repository
async fn count_files(repo_root: &Path) -> Result<usize> {
    let mut count = 0;
    count_files_recursive(repo_root, &mut count).await?;
    Ok(count)
}

/// Recursively count files
fn count_files_recursive<'a>(
    dir_path: &'a Path,
    count: &'a mut usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send + 'a>> {
    Box::pin(async move {
        let mut entries = fs::read_dir(dir_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }

            if entry.file_type().await?.is_dir() {
                count_files_recursive(&path, count).await?;
            } else {
                *count += 1;
            }
        }

        Ok(())
    })
}

/// Initialize refs directory (Git-compatible references)
async fn initialize_refs(forge_path: &Path) -> Result<()> {
    let refs_path = forge_path.join("refs");

    // Create refs subdirectories
    tokio::fs::create_dir_all(refs_path.join("heads")).await?;
    tokio::fs::create_dir_all(refs_path.join("tags")).await?;
    tokio::fs::create_dir_all(refs_path.join("remotes")).await?;

    // Get current Git branch if in a Git repo
    let head_ref = get_current_git_branch(forge_path.parent().unwrap())
        .await
        .unwrap_or_else(|_| "main".to_string());

    // Create HEAD file pointing to current branch
    let head_content = format!("ref: refs/heads/{}", head_ref);
    tokio::fs::write(forge_path.join("HEAD"), head_content).await?;

    // Create branch ref with current commit (initial)
    let branch_ref_path = refs_path.join("heads").join(&head_ref);
    let initial_commit = format!("{}-init", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    tokio::fs::write(&branch_ref_path, initial_commit).await?;

    println!("  📍 Initialized refs: HEAD -> refs/heads/{}", head_ref);

    Ok(())
}

/// Initialize logs directory (operation audit trail)
async fn initialize_logs(forge_path: &Path) -> Result<()> {
    let logs_path = forge_path.join("logs");

    // Create logs subdirectories
    tokio::fs::create_dir_all(logs_path.join("refs")).await?;

    // Create initial log entry
    let log_entry = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "action": "init",
        "message": "Repository initialized",
        "actor": std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "unknown".to_string()),
    });

    let log_file = logs_path.join("HEAD");
    tokio::fs::write(log_file, serde_json::to_string_pretty(&log_entry)?).await?;

    println!("  📝 Initialized logs: operation audit trail");

    Ok(())
}

/// Initialize context directory (AI context and annotations)
async fn initialize_context(forge_path: &Path) -> Result<()> {
    let context_path = forge_path.join("context");

    // Create context subdirectories
    tokio::fs::create_dir_all(context_path.join("discussions")).await?;
    tokio::fs::create_dir_all(context_path.join("annotations")).await?;
    tokio::fs::create_dir_all(context_path.join("ai_sessions")).await?;

    // Create initial context metadata
    let context_meta = serde_json::json!({
        "version": "1.0",
        "initialized_at": chrono::Utc::now().to_rfc3339(),
        "features": {
            "ai_discussions": true,
            "code_annotations": true,
            "anchor_tracking": true,
        }
    });

    let meta_file = context_path.join("metadata.json");
    tokio::fs::write(meta_file, serde_json::to_string_pretty(&context_meta)?).await?;

    println!("  💬 Initialized context: AI discussions and annotations");

    Ok(())
}

/// Get current Git branch
async fn get_current_git_branch(repo_path: &Path) -> Result<String> {
    use std::process::Command;

    let output = Command::new("git")
        .args(&["branch", "--show-current"])
        .current_dir(repo_path)
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Ok(branch);
        }
    }

    // Fallback: try reading .git/HEAD
    let git_head = repo_path.join(".git/HEAD");
    if git_head.exists() {
        let content = tokio::fs::read_to_string(git_head).await?;
        if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
            return Ok(branch.trim().to_string());
        }
    }

    anyhow::bail!("Not in a Git repository")
}
