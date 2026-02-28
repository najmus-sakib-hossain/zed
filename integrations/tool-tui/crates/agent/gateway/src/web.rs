//! Web UI server — static file serving and dashboard API.
//!
//! Serves the DX Agent control panel at `http://localhost:PORT/`.
//! Provides REST endpoints for the dashboard, webchat, configuration,
//! log viewer, and skill manager.

use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header},
    response::{Html, IntoResponse},
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

use crate::server::GatewayState;

/// Serve the main dashboard HTML page
pub async fn dashboard_handler() -> impl IntoResponse {
    Html(DASHBOARD_HTML)
}

/// API: Get dashboard data
pub async fn dashboard_api_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let uptime = (chrono::Utc::now() - state.start_time).num_seconds();

    Json(json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_secs": uptime,
        "connections": state.clients.len(),
        "sessions": state.session_store.count().unwrap_or(0),
        "config": {
            "host": state.config.server.host,
            "port": state.config.server.port,
            "auth_required": state.config.auth.required,
            "rate_limit_enabled": state.config.rate_limit.enabled,
            "max_connections": state.config.server.max_connections,
        }
    }))
}

/// API: WebChat endpoint — handles chat messages from the web UI
pub async fn webchat_handler(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let message = payload.get("message").and_then(|v| v.as_str()).unwrap_or("");

    if message.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "message is required"})));
    }

    // Broadcast as a gateway event for any connected agent
    let event = dx_agent_protocol::GatewayEvent::new(
        "webchat.message",
        json!({
            "message": message,
            "source": "webchat",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }),
    );
    let _ = state.event_tx.send(event);

    (
        StatusCode::OK,
        Json(json!({
            "status": "sent",
            "message": message,
        })),
    )
}

/// API: Get gateway configuration (sanitized — no secrets)
pub async fn config_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    Json(json!({
        "server": {
            "host": state.config.server.host,
            "port": state.config.server.port,
            "max_message_size": state.config.server.max_message_size,
            "max_connections": state.config.server.max_connections,
            "heartbeat_interval_secs": state.config.server.heartbeat_interval_secs,
            "cors_enabled": state.config.server.cors_enabled,
        },
        "auth": {
            "required": state.config.auth.required,
            "token_expiry_secs": state.config.auth.token_expiry_secs,
            "api_key_count": state.config.auth.api_keys.len(),
        },
        "rate_limit": {
            "enabled": state.config.rate_limit.enabled,
            "max_requests": state.config.rate_limit.max_requests,
            "window_secs": state.config.rate_limit.window_secs,
        },
        "logging": {
            "level": state.config.logging.level,
            "json": state.config.logging.json,
        }
    }))
}

/// API: Get connected clients
pub async fn clients_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let clients: Vec<serde_json::Value> = state
        .clients
        .iter()
        .map(|entry| {
            let c = entry.value();
            json!({
                "id": c.id,
                "addr": c.addr.to_string(),
                "connected_at": c.connected_at.to_rfc3339(),
                "session_id": c.session_id,
                "authenticated": c.authenticated,
            })
        })
        .collect();

    Json(json!({ "clients": clients }))
}

/// API: Read recent logs (for web log viewer)
pub async fn logs_handler(State(state): State<Arc<GatewayState>>) -> impl IntoResponse {
    let max_lines = 200usize;
    let lines = if let Some(path) = state.config.logging.file.clone() {
        read_tail_lines(path, max_lines).unwrap_or_else(|_| vec![])
    } else {
        vec!["File logging not configured in gateway config".to_string()]
    };

    Json(json!({
        "lines": lines,
        "count": lines.len()
    }))
}

/// API: List bundled + workspace skills for web skill manager
pub async fn skills_handler() -> impl IntoResponse {
    let bundled =
        discover_skills(vec![PathBuf::from("crates/cli/skills"), PathBuf::from("skills")]);

    let workspace = discover_workspace_skills();

    Json(json!({
        "bundled": bundled,
        "workspace": workspace,
        "total": bundled.len() + workspace.len()
    }))
}

fn read_tail_lines(path: PathBuf, max_lines: usize) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    if lines.len() > max_lines {
        lines = lines.split_off(lines.len() - max_lines);
    }
    Ok(lines)
}

fn discover_skills(candidates: Vec<PathBuf>) -> Vec<String> {
    for dir in candidates {
        if dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                let mut skills = vec![];
                for entry in entries.flatten() {
                    if let Ok(ft) = entry.file_type() {
                        if ft.is_dir() {
                            if let Some(name) = entry.file_name().to_str() {
                                skills.push(name.to_string());
                            }
                        }
                    }
                }
                skills.sort();
                if !skills.is_empty() {
                    return skills;
                }
            }
        }
    }
    vec![]
}

fn discover_workspace_skills() -> Vec<String> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let ws_dir = cwd.join(".dx").join("skills");
    if !ws_dir.exists() {
        return vec![];
    }

    if let Ok(entries) = std::fs::read_dir(ws_dir) {
        let mut skills = vec![];
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    skills.push(stem.to_string());
                }
            }
        }
        skills.sort();
        return skills;
    }

    vec![]
}

/// Serve CSS
pub async fn styles_handler() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, HeaderValue::from_static("text/css"))], DASHBOARD_CSS)
}

/// Embedded dashboard HTML
const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>DX Agent — Control Panel</title>
    <link rel="stylesheet" href="/ui/styles.css">
</head>
<body>
    <nav>
        <h1>⚡ DX Agent</h1>
        <span id="status-badge" class="badge">connecting...</span>
    </nav>
    <main>
        <section id="dashboard">
            <h2>Dashboard</h2>
            <div class="cards">
                <div class="card"><h3>Connections</h3><p id="conn-count">-</p></div>
                <div class="card"><h3>Sessions</h3><p id="sess-count">-</p></div>
                <div class="card"><h3>Uptime</h3><p id="uptime">-</p></div>
                <div class="card"><h3>Version</h3><p id="version">-</p></div>
            </div>
        </section>
        <section id="webchat">
            <h2>WebChat</h2>
            <div id="chat-log"></div>
            <form id="chat-form">
                <input type="text" id="chat-input" placeholder="Type a message..." autocomplete="off">
                <button type="submit">Send</button>
            </form>
        </section>
        <section id="logs">
            <h2>Log Viewer</h2>
            <pre id="logs-view"></pre>
        </section>
        <section id="skill-manager">
            <h2>Skill Manager</h2>
            <div class="cards">
                <div class="card"><h3>Bundled Skills</h3><p id="skills-bundled">-</p></div>
                <div class="card"><h3>Workspace Skills</h3><p id="skills-workspace">-</p></div>
                <div class="card"><h3>Total Skills</h3><p id="skills-total">-</p></div>
            </div>
        </section>
    </main>
    <script>
        async function refresh() {
            try {
                const res = await fetch('/api/v1/dashboard');
                const data = await res.json();
                document.getElementById('conn-count').textContent = data.connections;
                document.getElementById('sess-count').textContent = data.sessions;
                document.getElementById('version').textContent = 'v' + data.version;
                const h = Math.floor(data.uptime_secs / 3600);
                const m = Math.floor((data.uptime_secs % 3600) / 60);
                document.getElementById('uptime').textContent = h + 'h ' + m + 'm';
                document.getElementById('status-badge').textContent = 'online';
                document.getElementById('status-badge').className = 'badge online';

                const logsRes = await fetch('/api/v1/logs');
                const logsData = await logsRes.json();
                document.getElementById('logs-view').textContent = (logsData.lines || []).join('\n');

                const skillsRes = await fetch('/api/v1/skills');
                const skillsData = await skillsRes.json();
                document.getElementById('skills-bundled').textContent = (skillsData.bundled || []).length;
                document.getElementById('skills-workspace').textContent = (skillsData.workspace || []).length;
                document.getElementById('skills-total').textContent = skillsData.total || 0;
            } catch {
                document.getElementById('status-badge').textContent = 'offline';
                document.getElementById('status-badge').className = 'badge offline';
            }
        }
        refresh();
        setInterval(refresh, 5000);

        document.getElementById('chat-form').addEventListener('submit', async (e) => {
            e.preventDefault();
            const input = document.getElementById('chat-input');
            const msg = input.value.trim();
            if (!msg) return;
            input.value = '';
            const log = document.getElementById('chat-log');
            log.innerHTML += '<div class="msg user">' + msg + '</div>';
            log.scrollTop = log.scrollHeight;
            try {
                const res = await fetch('/api/v1/webchat', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ message: msg }),
                });
                const data = await res.json();
                log.innerHTML += '<div class="msg bot">Sent ✓</div>';
            } catch {
                log.innerHTML += '<div class="msg error">Failed to send</div>';
            }
            log.scrollTop = log.scrollHeight;
        });
    </script>
</body>
</html>"#;

/// Embedded dashboard CSS
const DASHBOARD_CSS: &str = r#"
:root { --bg: #0a0a0a; --card: #1a1a1a; --accent: #3b82f6; --text: #e5e5e5; --muted: #737373; }
* { margin: 0; padding: 0; box-sizing: border-box; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: var(--bg); color: var(--text); }
nav { display: flex; justify-content: space-between; align-items: center; padding: 1rem 2rem; border-bottom: 1px solid #222; }
nav h1 { font-size: 1.2rem; }
.badge { padding: 4px 12px; border-radius: 12px; font-size: 0.75rem; background: #333; }
.badge.online { background: #166534; color: #4ade80; }
.badge.offline { background: #7f1d1d; color: #fca5a5; }
main { max-width: 900px; margin: 2rem auto; padding: 0 1rem; }
h2 { margin-bottom: 1rem; color: var(--accent); }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 1rem; margin-bottom: 2rem; }
.card { background: var(--card); padding: 1.5rem; border-radius: 8px; border: 1px solid #222; }
.card h3 { font-size: 0.8rem; color: var(--muted); text-transform: uppercase; margin-bottom: 0.5rem; }
.card p { font-size: 1.5rem; font-weight: 600; }
#webchat { margin-top: 2rem; }
#chat-log { background: var(--card); border-radius: 8px; padding: 1rem; min-height: 200px; max-height: 400px; overflow-y: auto; margin-bottom: 1rem; border: 1px solid #222; }
.msg { padding: 0.5rem; margin: 0.25rem 0; border-radius: 4px; }
.msg.user { background: #1e3a5f; text-align: right; }
.msg.bot { background: #1a2e1a; }
.msg.error { background: #3a1a1a; color: #fca5a5; }
#chat-form { display: flex; gap: 0.5rem; }
#chat-input { flex: 1; padding: 0.75rem; background: var(--card); border: 1px solid #333; border-radius: 6px; color: var(--text); font-size: 1rem; }
#chat-input:focus { outline: none; border-color: var(--accent); }
#logs { margin-top: 2rem; }
#logs-view { background: var(--card); border: 1px solid #222; border-radius: 8px; padding: 1rem; min-height: 180px; max-height: 280px; overflow: auto; color: #9ca3af; }
#skill-manager { margin-top: 2rem; }
button { padding: 0.75rem 1.5rem; background: var(--accent); color: white; border: none; border-radius: 6px; cursor: pointer; font-size: 1rem; }
button:hover { opacity: 0.9; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_dashboard_html_not_empty() {
        assert!(DASHBOARD_HTML.contains("DX Agent"));
        assert!(DASHBOARD_HTML.contains("webchat"));
        assert!(DASHBOARD_HTML.contains("skill-manager"));
        assert!(DASHBOARD_HTML.contains("Log Viewer"));
    }

    #[test]
    fn test_dashboard_css_not_empty() {
        assert!(DASHBOARD_CSS.contains("--accent"));
    }

    #[test]
    fn test_discover_skills_from_directory() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join("skill-a")).expect("create skill-a");
        fs::create_dir_all(root.join("skill-b")).expect("create skill-b");

        let skills = discover_skills(vec![root.to_path_buf()]);
        assert_eq!(skills.len(), 2);
        assert!(skills.contains(&"skill-a".to_string()));
        assert!(skills.contains(&"skill-b".to_string()));
    }
}
