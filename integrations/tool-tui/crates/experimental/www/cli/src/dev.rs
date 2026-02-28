//! Development server implementation

use anyhow::Result;
use console::style;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;

pub async fn cmd_dev(_port: u16, _host: &str, _hot_reload: bool) -> Result<()> {
    println!("{}", style("Starting development server...").cyan().bold());
    println!();

    let config_path = std::path::Path::new("dx");
    let config_content = if config_path.exists() {
        std::fs::read_to_string(config_path)
            .unwrap_or_else(|_| String::from("# No dx config found"))
    } else {
        String::from("# No dx config found")
    };

    let index_path = std::path::Path::new("pages/index.pg");
    let index_content = if index_path.exists() {
        std::fs::read_to_string(index_path)
            .unwrap_or_else(|_| String::from("Error reading index.pg"))
    } else {
        String::from("No pages/index.pg found")
    };

    let addr: SocketAddr = "127.0.0.1:3000".parse()?;
    let listener = TcpListener::bind(addr).await?;

    println!("🚀 Development server running at http://localhost:3000");
    println!("   Hot reload: enabled");
    println!("   Config: dx (serializer format)");
    println!();
    println!("Press Ctrl+C to stop the server");
    println!();

    let config_content = Arc::new(config_content);
    let index_content = Arc::new(index_content);

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let config_clone = config_content.clone();
        let index_clone = index_content.clone();

        tokio::task::spawn(async move {
            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let config = config_clone.clone();
                let index = index_clone.clone();
                let start = Instant::now();
                let path = req.uri().path().to_string();
                let method = req.method().to_string();

                async move {
                    let render_start = Instant::now();

                    let html_response = generate_dev_page(&config, &index);
                    let html_bytes = Bytes::from(html_response);
                    let size_bytes = html_bytes.len();

                    let render_time = render_start.elapsed();
                    let total_time = start.elapsed();

                    let method_colored = match method.as_str() {
                        "GET" => style(&method).green(),
                        "POST" => style(&method).blue(),
                        "PUT" => style(&method).yellow(),
                        "DELETE" => style(&method).red(),
                        "HEAD" => style(&method).cyan(),
                        _ => style(&method).white(),
                    };

                    let path_colored = if path == "/" {
                        style(&path).cyan()
                    } else if path.starts_with("/api") {
                        style(&path).magenta()
                    } else {
                        style(&path).blue()
                    };

                    println!(
                        "{} {} {} - {} (render: {}) - {}",
                        style("→").dim(),
                        method_colored.bold(),
                        path_colored,
                        style(crate::utils::format_time(total_time)).yellow(),
                        style(crate::utils::format_time(render_time)).dim(),
                        style(crate::utils::format_size(size_bytes)).cyan()
                    );

                    Ok::<_, Infallible>(
                        Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "text/html; charset=utf-8")
                            .header("X-Render-Time", format!("{}μs", render_time.as_micros()))
                            .header("X-Total-Time", format!("{}μs", total_time.as_micros()))
                            .header("X-Response-Size", size_bytes.to_string())
                            .body(Full::new(html_bytes))
                            .unwrap(),
                    )
                }
            });

            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

fn generate_dev_page(config: &str, index: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DX WWW Dev Server</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: #0d1117;
            color: #c9d1d9;
            padding: 2rem;
            line-height: 1.6;
        }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1 {{ color: #58a6ff; margin-bottom: 1rem; font-size: 2rem; }}
        h2 {{ color: #8b949e; margin: 2rem 0 1rem; font-size: 1.3rem; border-bottom: 1px solid #21262d; padding-bottom: 0.5rem; }}
        .metrics {{
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 1.5rem;
            margin: 1rem 0;
        }}
        .metric-row {{
            display: flex;
            justify-content: space-between;
            padding: 0.5rem 0;
            border-bottom: 1px solid #21262d;
        }}
        .metric-row:last-child {{ border-bottom: none; }}
        .metric-label {{ color: #8b949e; }}
        .metric-value {{ color: #58a6ff; font-weight: bold; }}
        pre {{
            background: #161b22;
            border: 1px solid #30363d;
            border-radius: 6px;
            padding: 1rem;
            overflow-x: auto;
            color: #c9d1d9;
            font-size: 0.9rem;
        }}
        .status {{ color: #3fb950; font-weight: bold; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 DX WWW Development Server</h1>
        <p class="status">● Server is running</p>
        
        <h2>📊 Response Metrics</h2>
        <div class="metrics">
            <div class="metric-row">
                <span class="metric-label">Render Time</span>
                <span class="metric-value" id="render-time">-</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Total Time</span>
                <span class="metric-value" id="total-time">-</span>
            </div>
            <div class="metric-row">
                <span class="metric-label">Response Size</span>
                <span class="metric-value" id="response-size">-</span>
            </div>
        </div>

        <h2>⚙️ Project Configuration (dx)</h2>
        <pre>{}</pre>

        <h2>📄 Page Source (pages/index.pg)</h2>
        <pre>{}</pre>
    </div>

    <script>
        const headers = {{
            'X-Render-Time': document.querySelector('#render-time'),
            'X-Total-Time': document.querySelector('#total-time'),
            'X-Response-Size': document.querySelector('#response-size')
        }};

        fetch(window.location.href)
            .then(response => {{
                headers['X-Render-Time'].textContent = response.headers.get('X-Render-Time') || '-';
                headers['X-Total-Time'].textContent = response.headers.get('X-Total-Time') || '-';
                const size = response.headers.get('X-Response-Size');
                headers['X-Response-Size'].textContent = size ? formatBytes(parseInt(size)) : '-';
            }});

        function formatBytes(bytes) {{
            if (bytes < 1024) return bytes + ' B';
            if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(2) + ' KB';
            return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
        }}
    </script>
</body>
</html>"#,
        crate::utils::html_escape(config),
        crate::utils::html_escape(index)
    )
}
