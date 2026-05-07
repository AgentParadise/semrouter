//! Quick-start: load a tiny in-memory corpus, route inputs, print decisions.
//!
//! Run with: `cargo run --example quickstart --release`
//! Requires the `fastembed` feature (default-on).

use semrouter::{
    config::RouterConfig,
    embedding::FastEmbedEmbedder,
    SemanticRouter,
};
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("semrouter-quickstart-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    let routes = dir.join("routes.jsonl");
    let mut f = std::fs::File::create(&routes)?;
    writeln!(f, r#"{{"id":"r1","route":"time","text":"what time is it","tags":[],"risk":"low"}}"#)?;
    writeln!(f, r#"{{"id":"r2","route":"time","text":"tell me the current time","tags":[],"risk":"low"}}"#)?;
    writeln!(f, r#"{{"id":"r3","route":"weather","text":"is it going to rain","tags":[],"risk":"low"}}"#)?;
    writeln!(f, r#"{{"id":"r4","route":"weather","text":"give me the forecast","tags":[],"risk":"low"}}"#)?;

    let hn_path = dir.join("hard_negatives.jsonl");
    std::fs::File::create(&hn_path)?;

    let mut config = RouterConfig::default_config();
    config.router.embedding_model = "fastembed/AllMiniLML6V2".into();
    config.router.minimum_score = 0.22;
    config.router.minimum_margin = 0.005;
    config.storage.hard_negatives_file = hn_path.to_string_lossy().into_owned();

    println!("Loading fastembed model (downloads ~23MB on first run)...");
    let embedder = Box::new(FastEmbedEmbedder::new()?);
    let router = SemanticRouter::load(config, &routes, embedder)?;

    for input in &["got the time", "will it rain today", "play music"] {
        let decision = router.route(input)?;
        println!(
            "\ninput:           {:?}\nselected_route:  {:?}\nstatus:          {}\ntop_score:       {:.3}",
            decision.input,
            decision.selected_route,
            decision.status,
            decision.confidence.top_score
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
    Ok(())
}
