//! DX Agent Tool System — 50 consolidated tools with 300+ actions.
//!
//! Every tool follows a unified `tool + action + params` interface.
//! The LLM picks 1 of 50 tools (not 300+), then specifies an action.
//!
//! # Categories
//!
//! | Category | Tools |
//! |----------|-------|
//! | I/O & System | file, search, shell, sandbox, system, http, network, config |
//! | Browser & Desktop | browser, desktop |
//! | Version Control | git, github |
//! | Code Intelligence | lsp, ast, analyze, refactor, format, lint, review |
//! | Execution & Quality | test, debug, profile, experiment |
//! | Data & Storage | database, data, document |
//! | AI & Memory | memory, context, llm, agent, spawn |
//! | Infrastructure | docker, kubernetes, infra, package, security |
//! | Project & Docs | project, docs, diagram, design |
//! | Communication | notify, tracker |
//! | Workflow | workflow, deploy |
//! | Agent Intelligence | monitor, media, i18n, compliance, migrate |

// Core types
pub mod definition;
pub mod registry;

// ── I/O & System ────────────────────────────────────────────
pub mod config;
pub mod file;
pub mod http;
pub mod network;
pub mod sandbox;
pub mod search;
pub mod shell;
pub mod system;

// ── Browser & Desktop ───────────────────────────────────────
pub mod browser;
pub mod desktop;

// ── Version Control ─────────────────────────────────────────
pub mod git;
pub mod github;

// ── Code Intelligence ───────────────────────────────────────
pub mod analyze;
pub mod ast;
pub mod format;
pub mod lint;
pub mod lsp;
pub mod refactor;
pub mod review;

// ── Execution & Quality ─────────────────────────────────────
pub mod debug;
pub mod experiment;
pub mod profile;
pub mod testing;

// ── Data & Storage ──────────────────────────────────────────
pub mod data;
pub mod database;
pub mod document;

// ── AI & Memory ─────────────────────────────────────────────
pub mod agent;
pub mod context;
pub mod llm;
pub mod memory;
pub mod spawn;

// ── Infrastructure ──────────────────────────────────────────
pub mod docker;
pub mod infra;
pub mod kubernetes;
pub mod package;
pub mod security;

// ── Project & Docs ──────────────────────────────────────────
pub mod design;
pub mod diagram;
pub mod docs;
pub mod project;

// ── Communication ───────────────────────────────────────────
pub mod notify;
pub mod tracker;

// ── Workflow & Deployment ───────────────────────────────────
pub mod deploy;
pub mod workflow;

// ── Monitoring & Specialized ────────────────────────────────
pub mod compliance;
pub mod i18n;
pub mod media;
pub mod migrate;
pub mod monitor;

// Legacy compatibility (kept for existing consumers)
pub mod bash;
pub mod file_ops;
pub mod sessions;
pub mod web_search;

// Re-exports
pub use browser::BrowserTool;
pub use definition::{Tool, ToolCall, ToolDefinition, ToolParameter, ToolResult};
pub use registry::ToolRegistry;
