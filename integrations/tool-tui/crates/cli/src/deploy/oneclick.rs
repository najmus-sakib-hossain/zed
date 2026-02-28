//! One-Click Deployment Configuration Generator
//!
//! Generates deployment configurations for various platforms with minimal user input.
//! Supports Docker, Railway, Fly.io, Render, DigitalOcean App Platform, and VPS.

use std::collections::HashMap;
use std::path::Path;

/// Platform-specific deployment generator
pub struct OneClickDeploy {
    /// Project name
    pub project_name: String,
    /// Project root path
    pub project_root: String,
    /// Detected project type
    pub project_type: ProjectType,
    /// Environment variables
    pub env_vars: HashMap<String, String>,
}

/// Detected project type
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    /// Rust binary/library
    Rust { binary_name: String, has_wasm: bool },
    /// Node.js/TypeScript project
    Node {
        package_manager: String,
        framework: Option<String>,
    },
    /// Python project
    Python { framework: Option<String> },
    /// Static site
    Static { build_dir: String },
    /// DX www application
    DxWww { entry_point: String },
}

/// Deployment platform
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Docker,
    Railway,
    FlyIo,
    Render,
    DigitalOcean,
    AWS,
    GCP,
    Vercel,
    Netlify,
    VPS,
}

impl OneClickDeploy {
    /// Create new deployment generator
    pub fn new(project_name: &str, project_root: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            project_root: project_root.to_string(),
            project_type: ProjectType::Static {
                build_dir: "dist".into(),
            },
            env_vars: HashMap::new(),
        }
    }

    /// Detect project type from files
    pub fn detect_project_type(&mut self) -> &ProjectType {
        let root = Path::new(&self.project_root);

        // Check for Cargo.toml (Rust)
        if root.join("Cargo.toml").exists() {
            let has_wasm = root.join("src").join("lib.rs").exists();
            self.project_type = ProjectType::Rust {
                binary_name: self.project_name.clone(),
                has_wasm,
            };
        }
        // Check for package.json (Node)
        else if root.join("package.json").exists() {
            let pm = if root.join("pnpm-lock.yaml").exists() {
                "pnpm"
            } else if root.join("yarn.lock").exists() {
                "yarn"
            } else if root.join("bun.lockb").exists() {
                "bun"
            } else {
                "npm"
            };

            let framework = if root.join("next.config.js").exists()
                || root.join("next.config.mjs").exists()
            {
                Some("next".to_string())
            } else if root.join("vite.config.ts").exists() || root.join("vite.config.js").exists() {
                Some("vite".to_string())
            } else if root.join("astro.config.mjs").exists() {
                Some("astro".to_string())
            } else {
                None
            };

            self.project_type = ProjectType::Node {
                package_manager: pm.to_string(),
                framework,
            };
        }
        // Check for Python
        else if root.join("pyproject.toml").exists() || root.join("requirements.txt").exists() {
            let framework = if root.join("manage.py").exists() {
                Some("django".to_string())
            } else if root.join("app.py").exists() || root.join("main.py").exists() {
                Some("flask".to_string())
            } else {
                None
            };

            self.project_type = ProjectType::Python { framework };
        }
        // Check for DX www
        else if root.join("dx.config.toml").exists() {
            self.project_type = ProjectType::DxWww {
                entry_point: "pages/index.tsx".to_string(),
            };
        }

        &self.project_type
    }

    /// Add environment variable
    pub fn add_env(&mut self, key: &str, value: &str) {
        self.env_vars.insert(key.to_string(), value.to_string());
    }

    /// Generate all deployment configurations
    pub fn generate_all(&self) -> HashMap<Platform, String> {
        let mut configs = HashMap::new();

        configs.insert(Platform::Docker, self.generate_dockerfile());
        configs.insert(Platform::Railway, self.generate_railway_config());
        configs.insert(Platform::FlyIo, self.generate_fly_config());
        configs.insert(Platform::Render, self.generate_render_config());
        configs.insert(Platform::DigitalOcean, self.generate_digitalocean_config());

        configs
    }

    /// Generate Dockerfile
    pub fn generate_dockerfile(&self) -> String {
        match &self.project_type {
            ProjectType::Rust {
                binary_name,
                has_wasm,
            } => self.generate_rust_dockerfile(binary_name, *has_wasm),
            ProjectType::Node {
                package_manager,
                framework,
            } => self.generate_node_dockerfile(package_manager, framework.as_deref()),
            ProjectType::Python { framework } => {
                self.generate_python_dockerfile(framework.as_deref())
            }
            ProjectType::DxWww { .. } => self.generate_dx_dockerfile(),
            ProjectType::Static { build_dir } => self.generate_static_dockerfile(build_dir),
        }
    }

    fn generate_rust_dockerfile(&self, binary_name: &str, has_wasm: bool) -> String {
        let wasm_stage = if has_wasm {
            r#"
# WASM build stage
FROM rust:1.75-slim AS wasm-builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli wasm-opt
WORKDIR /app
COPY . .
RUN cargo build --release --target wasm32-unknown-unknown
RUN wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/*.wasm
RUN wasm-opt -Os -o pkg/optimized.wasm pkg/*.wasm
"#
        } else {
            ""
        };

        let copy_wasm = if has_wasm {
            "COPY --from=wasm-builder /app/pkg /app/pkg"
        } else {
            ""
        };

        format!(
            r#"# Multi-stage build for {binary_name}
# Generated by DX One-Click Deploy
{wasm_stage}
# Build stage
FROM rust:1.75-slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src ./src
RUN cargo build --release --locked

# Runtime stage
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -r -s /bin/false appuser
WORKDIR /app
COPY --from=builder /app/target/release/{binary_name} /app/{binary_name}
{copy_wasm}
RUN chown -R appuser:appuser /app
USER appuser

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${{PORT:-8080}}/health || exit 1

ENV PORT=8080
EXPOSE ${{PORT}}

CMD ["/app/{binary_name}"]
"#,
            binary_name = binary_name,
            wasm_stage = wasm_stage,
            copy_wasm = copy_wasm,
        )
    }

    fn generate_node_dockerfile(&self, package_manager: &str, framework: Option<&str>) -> String {
        let (install_cmd, build_cmd) = match package_manager {
            "pnpm" => ("pnpm install --frozen-lockfile", "pnpm build"),
            "yarn" => ("yarn install --frozen-lockfile", "yarn build"),
            "bun" => ("bun install --frozen-lockfile", "bun run build"),
            _ => ("npm ci", "npm run build"),
        };

        let runtime_image = match framework {
            Some("next") => "node:20-alpine",
            _ => "node:20-alpine",
        };

        let start_cmd = match framework {
            Some("next") => "node server.js",
            _ => "node dist/index.js",
        };

        format!(
            r#"# Multi-stage build for Node.js
# Generated by DX One-Click Deploy

# Build stage
FROM node:20-alpine AS builder
RUN corepack enable
WORKDIR /app
COPY package*.json pnpm-lock.yaml* yarn.lock* bun.lockb* ./
RUN {install_cmd}
COPY . .
RUN {build_cmd}

# Runtime stage
FROM {runtime_image} AS runtime
RUN addgroup -g 1001 -S nodejs && adduser -S nextjs -u 1001
WORKDIR /app
COPY --from=builder --chown=nextjs:nodejs /app/dist ./dist
COPY --from=builder --chown=nextjs:nodejs /app/package*.json ./
COPY --from=builder --chown=nextjs:nodejs /app/node_modules ./node_modules
USER nextjs

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:${{PORT:-3000}}/health || exit 1

ENV NODE_ENV=production
ENV PORT=3000
EXPOSE ${{PORT}}

CMD ["{start_cmd}"]
"#,
            install_cmd = install_cmd,
            build_cmd = build_cmd,
            runtime_image = runtime_image,
            start_cmd = start_cmd,
        )
    }

    fn generate_python_dockerfile(&self, framework: Option<&str>) -> String {
        let start_cmd = match framework {
            Some("django") => "gunicorn --bind 0.0.0.0:$PORT app.wsgi:application",
            Some("flask") => "gunicorn --bind 0.0.0.0:$PORT app:app",
            _ => "python main.py",
        };

        format!(
            r#"# Python application
# Generated by DX One-Click Deploy

FROM python:3.12-slim AS builder
WORKDIR /app
RUN pip install --no-cache-dir poetry || pip install --no-cache-dir pip-tools
COPY pyproject.toml poetry.lock* requirements*.txt ./
RUN if [ -f pyproject.toml ]; then \
        poetry export -f requirements.txt -o requirements.txt --without-hashes; \
    fi
RUN pip wheel --no-cache-dir --no-deps --wheel-dir /app/wheels -r requirements.txt

FROM python:3.12-slim AS runtime
RUN useradd -r -s /bin/false appuser
WORKDIR /app
COPY --from=builder /app/wheels /wheels
RUN pip install --no-cache /wheels/*
COPY . .
RUN chown -R appuser:appuser /app
USER appuser

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:${{PORT:-8000}}/health')"

ENV PORT=8000
EXPOSE ${{PORT}}

CMD ["{start_cmd}"]
"#,
            start_cmd = start_cmd,
        )
    }

    fn generate_dx_dockerfile(&self) -> String {
        format!(
            r#"# DX www application
# Generated by DX One-Click Deploy

# Build stage
FROM rust:1.75-slim AS builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli wasm-opt dx-cli
WORKDIR /app
COPY . .
RUN dx build --release

# Runtime stage
FROM nginx:alpine AS runtime
COPY --from=builder /app/dist /usr/share/nginx/html
COPY --from=builder /app/nginx.conf /etc/nginx/nginx.conf
RUN adduser -D -s /bin/false appuser && chown -R appuser:appuser /usr/share/nginx/html

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:80/health || exit 1

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
"#
        )
    }

    fn generate_static_dockerfile(&self, build_dir: &str) -> String {
        format!(
            r#"# Static site
# Generated by DX One-Click Deploy

FROM nginx:alpine
COPY {build_dir} /usr/share/nginx/html
RUN adduser -D -s /bin/false appuser

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:80/ || exit 1

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
"#,
            build_dir = build_dir,
        )
    }

    /// Generate Railway configuration
    pub fn generate_railway_config(&self) -> String {
        let (_build_cmd, _start_cmd) = self.get_build_start_commands();
        let env_section = self.format_env_vars_toml();

        format!(
            r#"# Railway configuration
# Generated by DX One-Click Deploy

[build]
builder = "dockerfile"
dockerfilePath = "Dockerfile"

[deploy]
healthcheckPath = "/health"
healthcheckTimeout = 30
restartPolicyType = "on-failure"
restartPolicyMaxRetries = 3

{env_section}
"#,
            env_section = env_section,
        )
    }

    /// Generate Fly.io configuration
    pub fn generate_fly_config(&self) -> String {
        let region = "iad"; // Default to US East
        let env_section = self.format_env_vars_toml();

        format!(
            r#"# Fly.io configuration
# Generated by DX One-Click Deploy

app = "{project_name}"
primary_region = "{region}"

[build]
  dockerfile = "Dockerfile"

[http_service]
  internal_port = 8080
  force_https = true
  auto_stop_machines = true
  auto_start_machines = true
  min_machines_running = 1

  [http_service.concurrency]
    type = "connections"
    hard_limit = 250
    soft_limit = 200

[[services]]
  protocol = "tcp"
  internal_port = 8080

  [[services.ports]]
    port = 80
    handlers = ["http"]
    force_https = true

  [[services.ports]]
    port = 443
    handlers = ["tls", "http"]

  [[services.tcp_checks]]
    interval = "15s"
    timeout = "2s"
    grace_period = "5s"

  [[services.http_checks]]
    interval = "10s"
    timeout = "2s"
    grace_period = "5s"
    method = "get"
    path = "/health"
    protocol = "http"

[checks]
  [checks.health]
    port = 8080
    type = "http"
    interval = "15s"
    timeout = "5s"
    grace_period = "10s"
    method = "GET"
    path = "/health"

{env_section}
"#,
            project_name = self.project_name,
            region = region,
            env_section = env_section,
        )
    }

    /// Generate Render configuration
    pub fn generate_render_config(&self) -> String {
        let (build_cmd, start_cmd) = self.get_build_start_commands();
        let runtime = match &self.project_type {
            ProjectType::Rust { .. } => "rust",
            ProjectType::Node { .. } => "node",
            ProjectType::Python { .. } => "python",
            _ => "docker",
        };

        let env_section = self.format_env_vars_yaml();

        format!(
            r#"# Render configuration
# Generated by DX One-Click Deploy

services:
  - type: web
    name: {project_name}
    runtime: {runtime}
    buildCommand: "{build_cmd}"
    startCommand: "{start_cmd}"
    healthCheckPath: /health
    autoDeploy: true
    plan: starter

    envVars:
      - key: PORT
        value: 10000
{env_section}
"#,
            project_name = self.project_name,
            runtime = runtime,
            build_cmd = build_cmd,
            start_cmd = start_cmd,
            env_section = env_section,
        )
    }

    /// Generate DigitalOcean App Platform configuration
    pub fn generate_digitalocean_config(&self) -> String {
        let (_build_cmd, _run_cmd) = self.get_build_start_commands();
        let env_section = self.format_env_vars_yaml();

        format!(
            r#"# DigitalOcean App Platform configuration
# Generated by DX One-Click Deploy

spec:
  name: {project_name}
  region: nyc
  
  services:
    - name: web
      dockerfile_path: Dockerfile
      source_dir: /
      http_port: 8080
      instance_size_slug: basic-xxs
      instance_count: 1
      
      health_check:
        http_path: /health
        initial_delay_seconds: 10
        period_seconds: 30
        timeout_seconds: 5
        success_threshold: 1
        failure_threshold: 3
      
      routes:
        - path: /
      
      envs:
        - key: PORT
          value: "8080"
{env_section}
"#,
            project_name = self.project_name,
            env_section = env_section,
        )
    }

    /// Generate docker-compose.yml
    pub fn generate_docker_compose(&self) -> String {
        let env_section = self.format_env_vars_compose();

        format!(
            r#"# Docker Compose configuration
# Generated by DX One-Click Deploy

version: '3.8'

services:
  app:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "${{PORT:-8080}}:8080"
    environment:
      - PORT=${{PORT:-8080}}
      - NODE_ENV=production
{env_section}
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
    restart: unless-stopped
    
  # Optional: Add database
  # db:
  #   image: postgres:15-alpine
  #   environment:
  #     POSTGRES_USER: ${{DB_USER:-app}}
  #     POSTGRES_PASSWORD: ${{DB_PASSWORD}}
  #     POSTGRES_DB: ${{DB_NAME:-app}}
  #   volumes:
  #     - db_data:/var/lib/postgresql/data
  #   healthcheck:
  #     test: ["CMD-SHELL", "pg_isready -U ${{DB_USER:-app}}"]
  #     interval: 10s
  #     timeout: 5s
  #     retries: 5

# volumes:
#   db_data:

networks:
  default:
    driver: bridge
"#,
            env_section = env_section,
        )
    }

    fn get_build_start_commands(&self) -> (String, String) {
        match &self.project_type {
            ProjectType::Rust { binary_name, .. } => {
                ("cargo build --release".into(), format!("./target/release/{}", binary_name))
            }
            ProjectType::Node {
                package_manager, ..
            } => {
                let build = match package_manager.as_str() {
                    "pnpm" => "pnpm build",
                    "yarn" => "yarn build",
                    "bun" => "bun run build",
                    _ => "npm run build",
                };
                let start = match package_manager.as_str() {
                    "pnpm" => "pnpm start",
                    "yarn" => "yarn start",
                    "bun" => "bun start",
                    _ => "npm start",
                };
                (build.into(), start.into())
            }
            ProjectType::Python { framework } => {
                let start = match framework.as_deref() {
                    Some("django") => "gunicorn app.wsgi:application",
                    Some("flask") => "gunicorn app:app",
                    _ => "python main.py",
                };
                ("pip install -r requirements.txt".into(), start.into())
            }
            ProjectType::DxWww { .. } => ("dx build --release".into(), "dx serve".into()),
            ProjectType::Static { .. } => ("echo 'No build needed'".into(), "nginx".into()),
        }
    }

    fn format_env_vars_toml(&self) -> String {
        if self.env_vars.is_empty() {
            return String::new();
        }

        let vars: Vec<String> =
            self.env_vars.iter().map(|(k, v)| format!("{} = \"{}\"", k, v)).collect();

        format!("[env]\n{}", vars.join("\n"))
    }

    fn format_env_vars_yaml(&self) -> String {
        if self.env_vars.is_empty() {
            return String::new();
        }

        self.env_vars
            .iter()
            .map(|(k, v)| format!("      - key: {}\n        value: \"{}\"", k, v))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_env_vars_compose(&self) -> String {
        if self.env_vars.is_empty() {
            return String::new();
        }

        let vars: Vec<String> =
            self.env_vars.iter().map(|(k, v)| format!("      - {}={}", k, v)).collect();

        vars.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rust_dockerfile() {
        let deploy = OneClickDeploy {
            project_name: "myapp".into(),
            project_root: ".".into(),
            project_type: ProjectType::Rust {
                binary_name: "myapp".into(),
                has_wasm: false,
            },
            env_vars: HashMap::new(),
        };

        let dockerfile = deploy.generate_dockerfile();
        assert!(dockerfile.contains("FROM rust:1.75-slim AS builder"));
        assert!(dockerfile.contains("cargo build --release"));
        assert!(dockerfile.contains("HEALTHCHECK"));
    }

    #[test]
    fn test_generate_node_dockerfile() {
        let deploy = OneClickDeploy {
            project_name: "myapp".into(),
            project_root: ".".into(),
            project_type: ProjectType::Node {
                package_manager: "pnpm".into(),
                framework: Some("next".into()),
            },
            env_vars: HashMap::new(),
        };

        let dockerfile = deploy.generate_dockerfile();
        assert!(dockerfile.contains("pnpm install"));
        assert!(dockerfile.contains("node:20-alpine"));
    }

    #[test]
    fn test_generate_fly_config() {
        let deploy = OneClickDeploy {
            project_name: "myapp".into(),
            project_root: ".".into(),
            project_type: ProjectType::Rust {
                binary_name: "myapp".into(),
                has_wasm: false,
            },
            env_vars: HashMap::new(),
        };

        let config = deploy.generate_fly_config();
        assert!(config.contains("app = \"myapp\""));
        assert!(config.contains("[[services]]"));
        assert!(config.contains("health"));
    }

    #[test]
    fn test_env_vars() {
        let mut deploy = OneClickDeploy::new("myapp", ".");
        deploy.add_env("DATABASE_URL", "postgres://...");
        deploy.add_env("API_KEY", "secret");

        let config = deploy.generate_railway_config();
        assert!(config.contains("DATABASE_URL"));
        assert!(config.contains("API_KEY"));
    }
}
