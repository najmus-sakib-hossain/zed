//! `forge auth <backend> [--token <value>]`
//!
//! Saves OAuth / personal-access tokens into the repo's auth.redb store.
//! Supported backends:
//!   youtube  pinterest  soundcloud  sketchfab  github
//!   gdrive   dropbox    mega        r2         all-free
use anyhow::{bail, Context, Result};
use crate::core::repository::Repository;
use crate::mirror::auth::{AuthStore, TokenBundle};
use std::io::{self, BufRead, Write};
use std::path::Path;

pub fn run(backend: &str, token: Option<&str>) -> Result<()> {
    let repo = Repository::discover(Path::new("."))?;
    let store = AuthStore::open(&repo.forge_dir)?;

    match backend {
        "youtube"    => auth_oauth2(&store, "youtube",    token)?,
        "pinterest"  => auth_oauth2(&store, "pinterest",  token)?,
        "soundcloud" => auth_oauth2(&store, "soundcloud", token)?,
        "sketchfab"  => auth_token(&store,  "sketchfab",  token)?,
        "github"     => auth_token(&store,  "github",     token)?,
        "gdrive"     => auth_oauth2(&store, "gdrive",     token)?,
        "dropbox"    => auth_oauth2(&store, "dropbox",    token)?,
        "mega"       => auth_basic(&store,  "mega")?,
        "r2"         => auth_r2(&store)?,
        "all-free"   => {
            for b in ["youtube", "pinterest", "soundcloud", "sketchfab", "github"] {
                println!("→ Authenticating {b}...");
                if let Err(e) = auth_oauth2(&store, b, None) {
                    eprintln!("  ⚠  {b}: {e}");
                } else {
                    println!("  ✓  {b}");
                }
            }
            println!("\nAll-free backends authenticated.");
            return Ok(());
        }
        other => bail!(
            "unknown backend: '{other}'\n\
             Available: youtube, pinterest, soundcloud, sketchfab, github,\n\
             \x20          gdrive, dropbox, mega, r2, all-free"
        ),
    }

    println!("✓  {backend} credentials saved.");
    Ok(())
}

// ---------- helpers ---------------------------------------------------------

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line).context("read input")?;
    Ok(line.trim().to_string())
}

/// Store a plain personal-access token (GitHub, Sketchfab …).
fn auth_token(store: &AuthStore, backend: &str, token: Option<&str>) -> Result<()> {
    let t = match token {
        Some(t) => t.to_string(),
        None => prompt(&format!("Paste your {backend} personal access token"))?,
    };
    store.save(backend, &TokenBundle {
        access_token: t,
        refresh_token: None,
        expires_at: None,
        extra: serde_json::Value::Null,
    })
    .context("save token")
}

/// Open browser for OAuth, then ask user to paste the resulting access token.
fn auth_oauth2(store: &AuthStore, backend: &str, token: Option<&str>) -> Result<()> {
    if let Some(t) = token {
        return store.save(backend, &TokenBundle {
            access_token: t.to_string(),
            refresh_token: None,
            expires_at: None,
            extra: serde_json::Value::Null,
        })
        .context("save token");
    }

    let oauth_hint: &[(&str, &str)] = &[
        ("youtube",    "https://accounts.google.com/o/oauth2/auth?scope=https://www.googleapis.com/auth/youtube.upload"),
        ("gdrive",     "https://accounts.google.com/o/oauth2/auth?scope=https://www.googleapis.com/auth/drive.file"),
        ("pinterest",  "https://www.pinterest.com/oauth/?scope=pins:write,boards:write\u{202c}"),
        ("soundcloud", "https://soundcloud.com/connect?scope=non-expiring"),
        ("dropbox",    "https://www.dropbox.com/oauth2/authorize"),
    ];

    if let Some((_, url)) = oauth_hint.iter().find(|(b, _)| *b == backend) {
        println!("Opening browser for {backend} OAuth…");
        println!("  URL: {url}");
        let _ = open::that(url); // best-effort; ok if headless
    }

    auth_token(store, backend, None)
}

/// Email + password (Mega).
fn auth_basic(store: &AuthStore, backend: &str) -> Result<()> {
    let email = prompt(&format!("{backend} email"))?;
    let pass  = prompt(&format!("{backend} password"))?;
    store.save(backend, &TokenBundle {
        access_token: email,
        refresh_token: Some(pass),
        expires_at: None,
        extra: serde_json::Value::Null,
    })
    .context("save credentials")
}

/// R2 / S3-compatible: key + secret + bucket + endpoint.
fn auth_r2(store: &AuthStore) -> Result<()> {
    let key      = prompt("R2 Access Key ID")?;
    let secret   = prompt("R2 Secret Access Key")?;
    let bucket   = prompt("Bucket name")?;
    let endpoint = prompt("Endpoint URL (e.g. https://ACCOUNT.r2.cloudflarestorage.com)")?;

    store.save("r2", &TokenBundle {
        access_token: key.clone(),
        refresh_token: None,
        expires_at: None,
        extra: serde_json::json!({
            "access_key_id":     key,
            "secret_access_key": secret,
            "bucket":            bucket,
            "endpoint":          endpoint,
        }),
    })
    .context("save r2 credentials")
}
