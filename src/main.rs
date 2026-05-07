#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use semrouter::{
    config::RouterConfig,
    embedding::{EmbeddingProvider, FastEmbedEmbedder},
    eval::{load_eval_cases, run_eval, EvalMetrics},
    experiment::ExperimentResult,
    SemanticRouter,
};

#[derive(Debug, Clone, ValueEnum)]
enum EmbedderType {
    Fastembed,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Parser)]
#[command(
    name = "semrouter",
    version,
    about = "Semantic vector router for agent/model/workflow dispatch"
)]
struct Cli {
    /// Path to router.toml config file
    #[arg(long, default_value = "router.toml")]
    config: PathBuf,

    /// Path to routes.jsonl examples file
    #[arg(long, default_value = "routes.jsonl")]
    routes: PathBuf,

    /// Embedder backend: fastembed (local ONNX, default)
    #[arg(long, default_value = "fastembed", value_enum)]
    embedder: EmbedderType,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Route input text and return a JSON decision
    Route {
        /// Input text to route
        input: String,

        /// Output compact JSON (no pretty printing)
        #[arg(long)]
        compact: bool,
    },
    /// List all loaded routes
    Routes,
    /// Show router info (config, example count)
    Info,
    /// Evaluate routing quality against a labelled test set
    Eval {
        /// Path to eval.jsonl (text + expected_route pairs)
        #[arg(long, default_value = "eval.jsonl")]
        eval_file: PathBuf,

        /// Output format: text (human-readable) or json (machine-readable)
        #[arg(long, default_value = "text", value_enum)]
        format: OutputFormat,

        /// Save experiment result to experiments/ directory
        #[arg(long)]
        save_experiment: bool,

        /// Path to thresholds.toml. If set, exit non-zero when any threshold fails.
        #[arg(long)]
        thresholds: Option<PathBuf>,
    },
}

fn build_embedder(embedder_type: &EmbedderType) -> Result<Box<dyn EmbeddingProvider>, String> {
    match embedder_type {
        EmbedderType::Fastembed => FastEmbedEmbedder::new()
            .map(|e| Box::new(e) as Box<dyn EmbeddingProvider>)
            .map_err(|e| format!("Failed to create fastembed embedder: {e}")),
    }
}

fn embedder_label(t: &EmbedderType) -> &'static str {
    match t {
        EmbedderType::Fastembed => "fastembed",
    }
}

fn main() {
    let cli = Cli::parse();

    let config = if cli.config.exists() {
        match RouterConfig::load(&cli.config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error loading config: {e}");
                std::process::exit(1);
            }
        }
    } else {
        RouterConfig::default_config()
    };

    if !cli.routes.exists() {
        eprintln!("Routes file not found: {}", cli.routes.display());
        eprintln!("Create a routes.jsonl file with route examples.");
        std::process::exit(1);
    }

    let embedder = match build_embedder(&cli.embedder) {
        Ok(e) => e,
        Err(msg) => {
            eprintln!("Embedder error: {msg}");
            std::process::exit(1);
        }
    };

    let router = match SemanticRouter::load(config.clone(), &cli.routes, embedder) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error loading router: {e}");
            std::process::exit(1);
        }
    };

    match cli.command {
        Commands::Route { input, compact } => match router.route(&input) {
            Ok(decision) => {
                let json = if compact {
                    serde_json::to_string(&decision).unwrap()
                } else {
                    serde_json::to_string_pretty(&decision).unwrap()
                };
                println!("{json}");
            }
            Err(e) => {
                eprintln!("Routing error: {e}");
                std::process::exit(1);
            }
        },

        Commands::Routes => {
            let routes = router.route_names();
            println!("Loaded routes ({}):", routes.len());
            for r in &routes {
                println!("  - {r}");
            }
        }

        Commands::Info => {
            println!("semrouter v{}", env!("CARGO_PKG_VERSION"));
            println!("Config:          {}", cli.config.display());
            println!("Routes file:     {}", cli.routes.display());
            println!("Examples loaded: {}", router.example_count());
            println!("Routes:          {}", router.route_names().join(", "));
            println!("Embedder:        {}", embedder_label(&cli.embedder));
            println!("Scoring:         top-{} per route", config.router.top_k);
            println!("Min score:       {}", config.router.minimum_score);
            println!("Min margin:      {}", config.router.minimum_margin);
        }

        Commands::Eval {
            eval_file,
            format,
            save_experiment,
            thresholds,
        } => {
            if !eval_file.exists() {
                eprintln!("Eval file not found: {}", eval_file.display());
                std::process::exit(1);
            }

            let cases = match load_eval_cases(&eval_file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error loading eval cases: {e}");
                    std::process::exit(1);
                }
            };

            if cases.is_empty() {
                eprintln!("No eval cases found in {}", eval_file.display());
                std::process::exit(1);
            }

            let metrics = run_eval(&router, &cases);

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&metrics).unwrap());
                }
                OutputFormat::Text => {
                    print_eval_metrics(&metrics);
                }
            }

            if save_experiment {
                let config_json = serde_json::to_value(&config).unwrap_or(serde_json::Value::Null);
                let result = ExperimentResult::from_eval(
                    &metrics,
                    embedder_label(&cli.embedder),
                    config_json,
                );
                match result.save(Path::new("experiments")) {
                    Ok(path) => eprintln!("Experiment saved to {}", path.display()),
                    Err(e) => eprintln!("Warning: could not save experiment: {e}"),
                }
            }

            // Threshold gating: exit non-zero on any breached floor/ceiling
            if let Some(thresholds_path) = thresholds {
                use semrouter::testing::{check_thresholds_public, EvalReport, Thresholds};
                let s = match std::fs::read_to_string(&thresholds_path) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Reading thresholds {}: {}", thresholds_path.display(), e);
                        std::process::exit(2);
                    }
                };
                let t: Thresholds = match toml::from_str(&s) {
                    Ok(t) => t,
                    Err(e) => {
                        eprintln!("Parsing thresholds {}: {}", thresholds_path.display(), e);
                        std::process::exit(2);
                    }
                };
                // EvalReport.load_ms is unused for CLI gating (the CLI doesn't
                // measure router load latency; only per-call route() latency).
                // Pass 0.0 — max_load_ms gating is intended for EvalSuite users only.
                let report = EvalReport {
                    metrics: metrics.clone(),
                    load_ms: 0.0,
                };
                let failures = check_thresholds_public(&t, &report);
                if !failures.is_empty() {
                    eprintln!();
                    eprintln!("Threshold failures:");
                    for f in &failures {
                        eprintln!("  - {f}");
                    }
                    std::process::exit(1);
                } else {
                    eprintln!();
                    eprintln!("All thresholds passed.");
                }
            }
        }
    }
}

fn print_eval_metrics(m: &EvalMetrics) {
    println!("Eval Results");
    println!("============");
    println!("Total:            {}", m.total);
    println!("Correct:          {}", m.correct);
    println!("Wrong:            {}", m.wrong);
    println!("Ambiguous:        {}", m.ambiguous);
    println!("Below threshold:  {}", m.below_threshold);
    println!("Accuracy:         {:.1}%", m.accuracy * 100.0);
    println!("Top-2 accuracy:   {:.1}%", m.top2_accuracy * 100.0);

    println!();
    println!("Per-Route Metrics");
    println!("-----------------");
    println!(
        "{:<30} {:>5}  {:>6}  {:>6}",
        "Route", "Prec", "Recall", "F1"
    );

    let mut routes: Vec<(&String, &semrouter::eval::RouteMetrics)> = m.per_route.iter().collect();
    routes.sort_by_key(|(name, _)| name.as_str());
    for (route, rm) in &routes {
        println!(
            "{:<30} {:>5.3}  {:>6.3}  {:>6.3}",
            route, rm.precision, rm.recall, rm.f1
        );
    }

    println!();
    println!("Latency (per route() call)");
    println!("--------------------------");
    println!("Samples:  {}", m.latency.samples);
    println!("Mean:     {:.3} ms", m.latency.mean_ms);
    println!("p50:      {:.3} ms", m.latency.p50_ms);
    println!("p95:      {:.3} ms", m.latency.p95_ms);
    println!("p99:      {:.3} ms", m.latency.p99_ms);
    println!(
        "Min/Max:  {:.3} / {:.3} ms",
        m.latency.min_ms, m.latency.max_ms
    );

    if !m.top_confusion.is_empty() {
        println!();
        println!("Top Confusion Pairs");
        println!("-------------------");
        println!("{:<25}  {:<25}  Count", "Expected", "Got");
        for entry in &m.top_confusion {
            println!("{:<25}  {:<25}  {}", entry.expected, entry.got, entry.count);
        }
    }
}
