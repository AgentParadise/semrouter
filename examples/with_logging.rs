//! Logging-aware example: routes inputs and writes one JSONL line per decision
//! to `/tmp/semrouter-demo-decisions.jsonl` so you can grep through them later.
//!
//! This is the recommended pattern for v0.1.x consumers who want decision
//! observability before semrouter v0.3.0 lands the built-in writer + tag CLI.
//! The schema below matches what semrouter v0.3.0 will produce, so your logs
//! are forward-compatible.
//!
//! Run with: `cargo run --example with_logging --release`
//! Requires the `fastembed` feature (default-on).

use semrouter::{
    config::RouterConfig,
    decision::RouteDecision,
    embedding::FastEmbedEmbedder,
    SemanticRouter,
};
use std::io::Write;
use std::path::Path;

/// Append a JSON line per decision. Best-effort — never panics on log error.
fn log_decision(log_path: &Path, input: &str, decision: &RouteDecision) {
    let payload = serde_json::json!({
        "decision_id": fake_uuid(),
        "timestamp": semrouter_compatible_iso8601(),
        "input": input,
        "selected_route": decision.selected_route,
        "status": decision.status,
        "confidence": decision.confidence,
        "candidates": decision.candidates.iter().take(3).collect::<Vec<_>>(),
    });

    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let _ = writeln!(f, "{}", payload);
    }
}

/// Tiny non-cryptographic id helper (good enough for local logs).
/// In a real consumer use the `uuid` crate.
fn fake_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("dec-{:x}", nanos)
}

/// Local copy of semrouter's internal time helper. Matches what
/// semrouter v0.3.0 will produce ("YYYY-MM-DDTHH:MM:SSZ").
fn semrouter_compatible_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Hinnant civil_from_days
    let days = (secs / 86_400) as i64;
    let secs_of_day = (secs % 86_400) as u32;
    let h = secs_of_day / 3600;
    let mi = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z / 146_097 } else { (z - 146_096) / 146_097 };
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = (y + if m <= 2 { 1 } else { 0 }) as i32;

    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up a tiny in-memory corpus so this example is self-contained.
    let dir = std::env::temp_dir().join(format!("semrouter-with-logging-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let routes = dir.join("routes.jsonl");
    {
        let mut f = std::fs::File::create(&routes)?;
        writeln!(f, r#"{{"id":"r1","route":"time","text":"what time is it","tags":[],"risk":"low"}}"#)?;
        writeln!(f, r#"{{"id":"r2","route":"time","text":"tell me the current time","tags":[],"risk":"low"}}"#)?;
        writeln!(f, r#"{{"id":"r3","route":"weather","text":"is it going to rain","tags":[],"risk":"low"}}"#)?;
        writeln!(f, r#"{{"id":"r4","route":"weather","text":"give me the forecast","tags":[],"risk":"low"}}"#)?;
        writeln!(f, r#"{{"id":"r5","route":"music","text":"play some music","tags":[],"risk":"low"}}"#)?;
        writeln!(f, r#"{{"id":"r6","route":"music","text":"start the playlist","tags":[],"risk":"low"}}"#)?;
    }
    std::fs::File::create(dir.join("hard_negatives.jsonl"))?;

    let mut config = RouterConfig::default_config();
    config.router.embedding_model = "fastembed/AllMiniLML6V2".into();
    config.router.minimum_score = 0.22;
    config.router.minimum_margin = 0.005;
    config.storage.hard_negatives_file = dir
        .join("hard_negatives.jsonl")
        .to_string_lossy()
        .into_owned();

    println!("Loading fastembed model (downloads ~23MB on first run)...");
    let embedder = Box::new(FastEmbedEmbedder::new()?);
    let router = SemanticRouter::load(config, &routes, embedder)?;

    let log_path = std::env::temp_dir().join("semrouter-demo-decisions.jsonl");
    println!("Logging to {}\n", log_path.display());

    // Truncate the log file at the start of each run for demo cleanliness.
    let _ = std::fs::remove_file(&log_path);

    let inputs = [
        "got the time",         // → time
        "will it rain today",   // → weather
        "play that song",       // → music
        "what's 42 squared",    // → below_threshold (no example matches)
    ];

    for input in &inputs {
        let decision = router.route(input)?;

        // Print to stdout for visibility.
        println!(
            "input:    {:?}\nrouted:   {:?}\nstatus:   {}\ntop:      {:.3}\n",
            decision.input,
            decision.selected_route,
            decision.status,
            decision.confidence.top_score
        );

        // Log to disk for later review/tagging.
        log_decision(&log_path, input, &decision);
    }

    // Show the resulting log.
    println!("--- {} ---", log_path.display());
    let log = std::fs::read_to_string(&log_path)?;
    println!("{}", log);

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
