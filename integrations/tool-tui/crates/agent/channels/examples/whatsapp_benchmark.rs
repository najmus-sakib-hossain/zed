use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use dx_agent_channels::traits::Channel;
use dx_agent_channels::whatsapp::{WhatsAppChannel, WhatsAppConfig};
use dx_agent_channels::{ChannelMessage, DeliveryStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchMode {
    Cloud,
    Baileys,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchKind {
    Live,
    Synthetic,
}

#[derive(Debug)]
struct BenchConfig {
    mode: BenchMode,
    to: String,
    iterations: usize,
    text: String,
    output: Option<PathBuf>,
    synthetic_fallback: bool,
}

#[derive(Debug)]
struct BenchResult {
    mode: &'static str,
    kind: &'static str,
    iterations: usize,
    successful: usize,
    failed: usize,
    total_ms: u128,
    avg_ms: f64,
    p95_ms: f64,
}

fn bench_kind_label(kind: BenchKind) -> &'static str {
    match kind {
        BenchKind::Live => "live",
        BenchKind::Synthetic => "synthetic",
    }
}

impl BenchConfig {
    fn from_env() -> Self {
        let mode = match std::env::var("DX_WHATSAPP_BENCH_MODE")
            .unwrap_or_else(|_| "both".to_string())
            .to_lowercase()
            .as_str()
        {
            "cloud" => BenchMode::Cloud,
            "baileys" => BenchMode::Baileys,
            _ => BenchMode::Both,
        };

        let to = std::env::var("DX_WHATSAPP_BENCH_TO").unwrap_or_else(|_| "15551234567".into());

        let iterations = std::env::var("DX_WHATSAPP_BENCH_ITERATIONS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(50);

        let text = std::env::var("DX_WHATSAPP_BENCH_TEXT")
            .unwrap_or_else(|_| "DX benchmark message".into());

        let output = std::env::var("DX_WHATSAPP_BENCH_OUTPUT").ok().map(PathBuf::from);

        let synthetic_fallback = std::env::var("DX_WHATSAPP_BENCH_SYNTHETIC_FALLBACK")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(true);

        Self {
            mode,
            to,
            iterations,
            text,
            output,
            synthetic_fallback,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = BenchConfig::from_env();
    let mut results = Vec::<BenchResult>::new();

    if matches!(config.mode, BenchMode::Cloud | BenchMode::Both) {
        match run_cloud_benchmark(&config).await {
            Ok(Some(result)) => results.push(result),
            Ok(None) if config.synthetic_fallback => {
                eprintln!("[bench] falling back to synthetic cloud benchmark");
                results.push(run_synthetic_benchmark("cloud", &config));
            }
            Ok(None) => {}
            Err(error) if config.synthetic_fallback => {
                eprintln!("[bench] cloud benchmark failed ({error}); using synthetic");
                results.push(run_synthetic_benchmark("cloud", &config));
            }
            Err(error) => return Err(error),
        }
    }

    if matches!(config.mode, BenchMode::Baileys | BenchMode::Both) {
        match run_baileys_benchmark(&config).await {
            Ok(Some(result)) => results.push(result),
            Ok(None) if config.synthetic_fallback => {
                eprintln!("[bench] falling back to synthetic baileys benchmark");
                results.push(run_synthetic_benchmark("baileys", &config));
            }
            Ok(None) => {}
            Err(error) if config.synthetic_fallback => {
                eprintln!("[bench] baileys benchmark failed ({error}); using synthetic");
                results.push(run_synthetic_benchmark("baileys", &config));
            }
            Err(error) => return Err(error),
        }
    }

    if results.is_empty() {
        anyhow::bail!(
            "No benchmark mode could run. Set live credentials or enable DX_WHATSAPP_BENCH_SYNTHETIC_FALLBACK=1"
        );
    }

    print_results(&results);

    if let Some(path) = config.output {
        let json = serde_json::to_string_pretty(&results_json(&results))?;
        std::fs::write(&path, json)?;
        println!("saved    {}", path.display());
    }

    Ok(())
}

async fn run_cloud_benchmark(config: &BenchConfig) -> Result<Option<BenchResult>> {
    let access_token = match std::env::var("DX_WHATSAPP_CLOUD_ACCESS_TOKEN") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("[skip] cloud benchmark: DX_WHATSAPP_CLOUD_ACCESS_TOKEN not set");
            return Ok(None);
        }
    };
    let phone_number_id = match std::env::var("DX_WHATSAPP_CLOUD_PHONE_NUMBER_ID") {
        Ok(v) => v,
        Err(_) => {
            eprintln!("[skip] cloud benchmark: DX_WHATSAPP_CLOUD_PHONE_NUMBER_ID not set");
            return Ok(None);
        }
    };

    let mut channel = WhatsAppChannel::new(WhatsAppConfig {
        access_token: Some(access_token),
        phone_number_id: Some(phone_number_id),
        verify_token: std::env::var("DX_WHATSAPP_CLOUD_VERIFY_TOKEN").ok(),
        business_account_id: std::env::var("DX_WHATSAPP_CLOUD_BUSINESS_ACCOUNT_ID").ok(),
        use_baileys: false,
    });

    run_mode("cloud", BenchKind::Live, &mut channel, config).await.map(Some)
}

async fn run_baileys_benchmark(config: &BenchConfig) -> Result<Option<BenchResult>> {
    let mut channel = WhatsAppChannel::new(WhatsAppConfig {
        access_token: None,
        phone_number_id: None,
        verify_token: None,
        business_account_id: None,
        use_baileys: true,
    });

    run_mode("baileys", BenchKind::Live, &mut channel, config).await.map(Some)
}

async fn run_mode(
    mode: &'static str,
    kind: BenchKind,
    channel: &mut dyn Channel,
    config: &BenchConfig,
) -> Result<BenchResult> {
    channel.connect().await?;

    let mut successful = 0usize;
    let mut failed = 0usize;
    let mut latencies_ns = Vec::with_capacity(config.iterations);

    let total_start = Instant::now();
    for i in 0..config.iterations {
        let content = format!("{} [{} {}]", config.text, mode, i + 1);
        let msg = ChannelMessage::text(config.to.clone(), content);

        let send_start = Instant::now();
        let status = channel.send(msg).await?;
        let elapsed_ns = send_start.elapsed().as_nanos() as f64;
        latencies_ns.push(elapsed_ns);

        match status {
            DeliveryStatus::Sent | DeliveryStatus::Delivered | DeliveryStatus::Read => {
                successful += 1
            }
            DeliveryStatus::Failed(_) | DeliveryStatus::Pending => failed += 1,
        }
    }
    let total_ms = total_start.elapsed().as_millis();

    channel.disconnect().await?;

    Ok(BenchResult {
        mode,
        kind: bench_kind_label(kind),
        iterations: config.iterations,
        successful,
        failed,
        total_ms,
        avg_ms: average_ms(&latencies_ns),
        p95_ms: p95_ms(&latencies_ns),
    })
}

fn run_synthetic_benchmark(mode: &'static str, config: &BenchConfig) -> BenchResult {
    let kind = BenchKind::Synthetic;
    let mut latencies_ns = Vec::with_capacity(config.iterations);
    let total_start = Instant::now();

    for i in 0..config.iterations {
        let iter_start = Instant::now();
        let content = format!("{} [{} {}]", config.text, mode, i + 1);
        let _normalized = content
            .trim()
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric() || ch.is_whitespace())
            .collect::<String>();
        let _request = serde_json::json!({
            "id": i,
            "method": if mode == "cloud" { "send_cloud" } else { "send_baileys" },
            "to": config.to,
            "payload": content,
        });
        latencies_ns.push(iter_start.elapsed().as_nanos() as f64);
    }

    BenchResult {
        mode,
        kind: bench_kind_label(kind),
        iterations: config.iterations,
        successful: config.iterations,
        failed: 0,
        total_ms: total_start.elapsed().as_millis(),
        avg_ms: average_ms(&latencies_ns),
        p95_ms: p95_ms(&latencies_ns),
    }
}

fn average_ms(latencies_ns: &[f64]) -> f64 {
    if latencies_ns.is_empty() {
        return 0.0;
    }
    let avg_ns = latencies_ns.iter().sum::<f64>() / latencies_ns.len() as f64;
    avg_ns / 1_000_000.0
}

fn p95_ms(latencies_ns: &[f64]) -> f64 {
    if latencies_ns.is_empty() {
        return 0.0;
    }
    let mut sorted = latencies_ns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((sorted.len() - 1) as f64 * 0.95).round() as usize;
    sorted.get(index).copied().unwrap_or(0.0) / 1_000_000.0
}

fn print_results(results: &[BenchResult]) {
    println!("\nWhatsApp benchmark results");
    println!("mode      kind       iter  ok  fail  total_ms  avg_ms   p95_ms");
    for result in results {
        println!(
            "{:<8}  {:<9}  {:>4}  {:>2}  {:>4}  {:>8}  {:>7.3}  {:>7.3}",
            result.mode,
            result.kind,
            result.iterations,
            result.successful,
            result.failed,
            result.total_ms,
            result.avg_ms,
            result.p95_ms
        );
    }

    if let (Some(c), Some(b)) = (
        results.iter().find(|r| r.mode == "cloud"),
        results.iter().find(|r| r.mode == "baileys"),
    ) {
        if c.avg_ms > 0.0 && b.avg_ms > 0.0 {
            let (faster, speedup) = if c.avg_ms < b.avg_ms {
                ("cloud", b.avg_ms / c.avg_ms)
            } else {
                ("baileys", c.avg_ms / b.avg_ms)
            };
            println!("speedup   {faster} is {speedup:.2}x faster by avg_ms");
        }
    }
}

fn results_json(results: &[BenchResult]) -> serde_json::Value {
    serde_json::json!({
        "results": results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "mode": r.mode,
                    "kind": r.kind,
                    "iterations": r.iterations,
                    "successful": r.successful,
                    "failed": r.failed,
                    "total_ms": r.total_ms,
                    "avg_ms": r.avg_ms,
                    "p95_ms": r.p95_ms,
                })
            })
            .collect::<Vec<_>>()
    })
}
