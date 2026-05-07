//! Public test harness for consumer corpora.
//!
//! Each consumer keeps its own `routes.jsonl` + `eval.jsonl` + `router.toml` +
//! `thresholds.toml` in a fixture directory and asserts that semrouter meets the
//! floor in their own `#[test]`:
//!
//! ```ignore
//! use semrouter::testing::EvalSuite;
//!
//! #[test]
//! fn voice_assistant_corpus_meets_quality_bar() {
//!     EvalSuite::from_dir("tests/fixtures/voice-assistant")
//!         .unwrap()
//!         .assert_passes();
//! }
//! ```

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::config::RouterConfig;
use crate::embedding::{EmbeddingProvider, MockEmbedder};
#[cfg(feature = "fastembed")]
use crate::embedding::FastEmbedEmbedder;
use crate::eval::{load_eval_cases, run_eval, EvalMetrics};
use crate::SemanticRouter;

/// Minimum/maximum bars an eval run must meet. All fields are optional —
/// only set keys are enforced.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct Thresholds {
    /// Minimum fraction of eval cases routed correctly (0.0–1.0).
    pub min_accuracy: Option<f32>,
    /// Minimum top-2 accuracy (correct route in top-2 candidates).
    pub min_top2_accuracy: Option<f32>,
    /// Minimum per-route F1 score; checked against every route in the corpus.
    pub min_per_route_f1: Option<f32>,
    /// Maximum median latency in milliseconds.
    pub max_p50_ms: Option<f64>,
    /// Maximum 95th-percentile latency in milliseconds.
    pub max_p95_ms: Option<f64>,
    /// Maximum 99th-percentile latency in milliseconds.
    pub max_p99_ms: Option<f64>,
    /// Maximum router load time in milliseconds.
    pub max_load_ms: Option<f64>,
}

/// The result of a successful eval run.
#[derive(Debug, Clone)]
pub struct EvalReport {
    /// Accuracy, per-route F1, confusion matrix, latency percentiles, etc.
    pub metrics: EvalMetrics,
    /// Wall-clock time to load the router (embed all examples), in milliseconds.
    pub load_ms: f64,
}

/// Returned by [`EvalSuite::evaluate`] when one or more thresholds are violated.
#[derive(Debug, Clone)]
pub struct FailureReport {
    /// Human-readable description of each threshold violation.
    pub failures: Vec<String>,
    /// The underlying eval report that triggered the failure.
    pub report: EvalReport,
}

impl std::fmt::Display for FailureReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "EvalSuite failed {} threshold(s):", self.failures.len())?;
        for line in &self.failures {
            writeln!(f, "  - {line}")?;
        }
        Ok(())
    }
}

/// A directory-loaded eval suite.
///
/// Reads `router.toml`, `routes.jsonl`, `eval.jsonl`, and optionally
/// `thresholds.toml` from the given directory. Storage paths in `router.toml`
/// are resolved relative to the fixture directory so the suite is self-contained.
pub struct EvalSuite {
    dir: PathBuf,
    config: RouterConfig,
    thresholds: Thresholds,
}

impl EvalSuite {
    /// Load an eval suite from a fixture directory.
    ///
    /// Fails if `router.toml` is missing or malformed, or if `thresholds.toml`
    /// exists but cannot be parsed. A missing `thresholds.toml` is allowed and
    /// produces an all-`None` [`Thresholds`] (no checks enforced).
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, String> {
        let dir = dir.as_ref().to_path_buf();
        let config = RouterConfig::load(&dir.join("router.toml"))
            .map_err(|e| format!("loading router.toml: {e}"))?;
        let thresholds_path = dir.join("thresholds.toml");
        let thresholds: Thresholds = if thresholds_path.exists() {
            let s = std::fs::read_to_string(&thresholds_path)
                .map_err(|e| format!("reading thresholds.toml: {e}"))?;
            toml::from_str(&s).map_err(|e| format!("parsing thresholds.toml: {e}"))?
        } else {
            Thresholds::default()
        };
        Ok(Self {
            dir,
            config,
            thresholds,
        })
    }

    /// Run the eval and apply thresholds.
    ///
    /// Returns [`EvalReport`] on pass, or [`FailureReport`] if any threshold is
    /// violated. Consumes `self` — use [`assert_passes`](EvalSuite::assert_passes)
    /// for the common `#[test]` pattern.
    #[allow(clippy::result_large_err)] // FailureReport carries the full EvalReport by design
    pub fn evaluate(self) -> Result<EvalReport, FailureReport> {
        let report = self.run_inner();
        let failures = check_thresholds(&self.thresholds, &report);
        if failures.is_empty() {
            Ok(report)
        } else {
            Err(FailureReport { failures, report })
        }
    }

    /// Convenience for `#[test]` use: panics with a structured message on failure.
    pub fn assert_passes(self) -> EvalReport {
        match self.evaluate() {
            Ok(r) => r,
            Err(fr) => panic!("{}", fr),
        }
    }

    // Panics are intentional here: `run_inner` is called from `evaluate` /
    // `assert_passes`, both of which are designed to fail loudly inside `#[test]`.
    fn run_inner(&self) -> EvalReport {
        use std::time::Instant;

        // Resolve storage paths relative to the fixture dir so the suite is
        // self-contained regardless of the process working directory.
        let routes_path = self.dir.join(&self.config.storage.routes_file);
        let eval_path = self.dir.join("eval.jsonl");

        // routes_file is passed explicitly to SemanticRouter::load below, so it
        // doesn't need patching here. hard_negatives_file is read from config
        // by SemanticRouter::load itself, so patch it to an absolute path
        // anchored at the fixture dir.
        // TODO: SemanticRouter::load should resolve storage paths relative to
        // the config file's directory so callers don't need to pre-absolutize.
        let mut patched = self.config.clone();
        patched.storage.hard_negatives_file = self
            .dir
            .join(&self.config.storage.hard_negatives_file)
            .to_string_lossy()
            .into_owned();

        let embedder = build_embedder_from_config(&self.config);

        let load_t0 = Instant::now();
        let router = SemanticRouter::load(patched, &routes_path, embedder)
            .unwrap_or_else(|e| panic!("EvalSuite: failed to load router: {e}"));
        let load_ms = load_t0.elapsed().as_micros() as f64 / 1000.0;

        let cases = load_eval_cases(&eval_path)
            .unwrap_or_else(|e| panic!("EvalSuite: loading {}: {}", eval_path.display(), e));
        let metrics = run_eval(&router, &cases);

        EvalReport { metrics, load_ms }
    }
}

/// Build an embedder from the `embedding_model` field in config.
///
/// Supports `"mock"` and `"fastembed/..."`. Consumers needing a different
/// backend implement the public [`EmbeddingProvider`] trait and use the
/// lower-level [`SemanticRouter::load`] directly instead of [`EvalSuite::from_dir`].
fn build_embedder_from_config(config: &RouterConfig) -> Box<dyn EmbeddingProvider> {
    let model = config.router.embedding_model.as_str();
    if model == "mock" {
        Box::new(MockEmbedder::new())
    } else if model.starts_with("fastembed/") {
        #[cfg(feature = "fastembed")]
        {
            Box::new(
                FastEmbedEmbedder::new()
                    .unwrap_or_else(|e| panic!("EvalSuite: failed to init fastembed: {e}")),
            )
        }
        #[cfg(not(feature = "fastembed"))]
        {
            panic!(
                "EvalSuite: embedding_model {model:?} requires the `fastembed` feature. \
                 Enable it in your Cargo.toml or use a custom EmbeddingProvider."
            );
        }
    } else {
        panic!(
            "EvalSuite: unsupported embedding_model {model:?}. Use \"mock\" or \"fastembed/...\"."
        )
    }
}

/// Check all thresholds against the eval report. Returns a list of human-readable
/// failure strings; empty means all thresholds passed.
fn check_thresholds(t: &Thresholds, r: &EvalReport) -> Vec<String> {
    let m = &r.metrics;
    let mut failures = Vec::new();

    if let Some(min) = t.min_accuracy {
        if m.accuracy < min {
            failures.push(format!(
                "accuracy {:.3} < min_accuracy {:.3}",
                m.accuracy, min
            ));
        }
    }
    if let Some(min) = t.min_top2_accuracy {
        if m.top2_accuracy < min {
            failures.push(format!(
                "top2_accuracy {:.3} < min {:.3}",
                m.top2_accuracy, min
            ));
        }
    }
    if let Some(min) = t.min_per_route_f1 {
        for (route, rm) in &m.per_route {
            if rm.f1 < min {
                failures.push(format!(
                    "route '{}' f1 {:.3} < min_per_route_f1 {:.3}",
                    route, rm.f1, min
                ));
            }
        }
    }
    if let Some(max) = t.max_p50_ms {
        if m.latency.p50_ms > max {
            failures.push(format!("p50 {:.2}ms > max {:.2}ms", m.latency.p50_ms, max));
        }
    }
    if let Some(max) = t.max_p95_ms {
        if m.latency.p95_ms > max {
            failures.push(format!("p95 {:.2}ms > max {:.2}ms", m.latency.p95_ms, max));
        }
    }
    if let Some(max) = t.max_p99_ms {
        if m.latency.p99_ms > max {
            failures.push(format!("p99 {:.2}ms > max {:.2}ms", m.latency.p99_ms, max));
        }
    }
    if let Some(max) = t.max_load_ms {
        if r.load_ms > max {
            failures.push(format!("load {:.2}ms > max {:.2}ms", r.load_ms, max));
        }
    }
    failures
}

/// Public wrapper for the CLI / external callers to apply thresholds against
/// an in-hand `EvalReport`. The CLI uses this to gate `eval --thresholds <path>`.
pub fn check_thresholds_public(t: &Thresholds, r: &EvalReport) -> Vec<String> {
    check_thresholds(t, r)
}
