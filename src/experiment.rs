use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::RouterError;
use crate::eval::{EvalMetrics, LatencyMetrics, RouteMetrics};

/// A persisted record of a single evaluation run, saved to `experiments/`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// ISO-8601 timestamp when the experiment was recorded.
    pub timestamp: String,
    /// Name or identifier of the embedder used in this run.
    pub embedder: String,
    /// Total number of eval cases.
    pub total: usize,
    /// Number of cases routed correctly.
    pub correct: usize,
    /// Overall accuracy (`correct / total`).
    pub accuracy: f32,
    /// Per-route precision, recall, and F1 metrics.
    pub per_route: HashMap<String, RouteMetrics>,
    /// Routing latency statistics for this run.
    pub latency: LatencyMetrics,
    /// Snapshot of the router configuration used for this experiment.
    pub config_snapshot: serde_json::Value,
}

impl ExperimentResult {
    /// Construct an `ExperimentResult` from eval metrics, an embedder name, and a config snapshot.
    pub fn from_eval(
        metrics: &EvalMetrics,
        embedder: &str,
        config_snapshot: serde_json::Value,
    ) -> Self {
        Self {
            timestamp: crate::time_util::iso8601_now(),
            embedder: embedder.to_string(),
            total: metrics.total,
            correct: metrics.correct,
            accuracy: metrics.accuracy,
            per_route: metrics.per_route.clone(),
            latency: metrics.latency.clone(),
            config_snapshot,
        }
    }

    /// Serialize and write this result to a timestamped JSON file in `experiments_dir`.
    pub fn save(&self, experiments_dir: &Path) -> Result<std::path::PathBuf, RouterError> {
        std::fs::create_dir_all(experiments_dir)?;
        let ts = crate::time_util::compact_now();
        let path = experiments_dir.join(format!("experiment_{ts}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(path)
    }
}
