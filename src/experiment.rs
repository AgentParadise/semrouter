use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::RouterError;
use crate::eval::{EvalMetrics, LatencyMetrics, RouteMetrics};

#[derive(Debug, Serialize, Deserialize)]
pub struct ExperimentResult {
    pub timestamp: String,
    pub embedder: String,
    pub total: usize,
    pub correct: usize,
    pub accuracy: f32,
    pub per_route: HashMap<String, RouteMetrics>,
    pub latency: LatencyMetrics,
    pub config_snapshot: serde_json::Value,
}

impl ExperimentResult {
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

    pub fn save(&self, experiments_dir: &Path) -> Result<std::path::PathBuf, RouterError> {
        std::fs::create_dir_all(experiments_dir)?;
        let ts = crate::time_util::compact_now();
        let path = experiments_dir.join(format!("experiment_{ts}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(path)
    }
}
