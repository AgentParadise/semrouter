use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::time::Instant;

use crate::decision::DecisionStatus;
use crate::error::RouterError;
use crate::SemanticRouter;

/// A single eval case: an input text and the route it should be dispatched to.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EvalCase {
    /// Input text to be routed.
    pub text: String,
    /// The route name that a correct router should return for this input.
    pub expected_route: String,
}

/// Precision/recall/F1 metrics for a single route.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RouteMetrics {
    /// True positives: inputs correctly routed to this route.
    pub tp: usize,
    /// False positives: inputs incorrectly routed to this route.
    pub fp: usize,
    /// False negatives: inputs belonging to this route that were missed.
    pub false_neg: usize,
    /// Precision: `tp / (tp + fp)`.
    pub precision: f32,
    /// Recall: `tp / (tp + false_neg)`.
    pub recall: f32,
    /// F1 score: harmonic mean of precision and recall.
    pub f1: f32,
}

/// Routing latency statistics across all eval cases.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct LatencyMetrics {
    /// Number of routing calls measured.
    pub samples: usize,
    /// Mean latency in milliseconds.
    pub mean_ms: f64,
    /// 50th-percentile latency in milliseconds.
    pub p50_ms: f64,
    /// 95th-percentile latency in milliseconds.
    pub p95_ms: f64,
    /// 99th-percentile latency in milliseconds.
    pub p99_ms: f64,
    /// Minimum observed latency in milliseconds.
    pub min_ms: f64,
    /// Maximum observed latency in milliseconds.
    pub max_ms: f64,
}

/// A single confusion-matrix entry: an (expected, got) pair and its count.
#[derive(Debug, Serialize, Clone)]
pub struct ConfusionEntry {
    /// The route that was expected.
    pub expected: String,
    /// The route that was actually returned.
    pub got: String,
    /// Number of times this confusion occurred.
    pub count: usize,
}

/// Aggregate evaluation metrics for a full eval run.
#[derive(Debug, Serialize, Clone)]
pub struct EvalMetrics {
    /// Total number of eval cases.
    pub total: usize,
    /// Number of cases routed to the correct route.
    pub correct: usize,
    /// Number of cases routed to the wrong route.
    pub wrong: usize,
    /// Number of cases where the router returned `Ambiguous`.
    pub ambiguous: usize,
    /// Number of cases where the router returned `BelowThreshold`.
    pub below_threshold: usize,
    /// Fraction of cases routed correctly (`correct / total`).
    pub accuracy: f32,
    /// Number of cases where the correct route appeared in the top-2 candidates.
    pub top2_correct: usize,
    /// Fraction of cases where the correct route was in the top-2 candidates.
    pub top2_accuracy: f32,
    /// Per-route precision, recall, and F1.
    pub per_route: HashMap<String, RouteMetrics>,
    /// Top-10 most frequent (expected, got) confusions.
    pub top_confusion: Vec<ConfusionEntry>,
    /// Routing latency statistics.
    pub latency: LatencyMetrics,
}

/// Load eval cases from a JSONL file, skipping blank lines and `//` comments.
pub fn load_eval_cases(path: &Path) -> Result<Vec<EvalCase>, RouterError> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut cases = Vec::new();

    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let case: EvalCase = serde_json::from_str(line)
            .map_err(|e| RouterError::Parse(format!("eval.jsonl line {}: {}", line_num + 1, e)))?;
        cases.push(case);
    }

    Ok(cases)
}

/// Run the router over all eval cases and return aggregate metrics.
pub fn run_eval(router: &SemanticRouter, cases: &[EvalCase]) -> EvalMetrics {
    let mut correct = 0usize;
    let mut wrong = 0usize;
    let mut ambiguous = 0usize;
    let mut below_threshold = 0usize;
    let mut top2_correct = 0usize;

    let mut tp_map: HashMap<String, usize> = HashMap::new();
    let mut fp_map: HashMap<String, usize> = HashMap::new();
    let mut fn_map: HashMap<String, usize> = HashMap::new();
    let mut confusion: HashMap<(String, String), usize> = HashMap::new();
    let mut latency_samples_us: Vec<u128> = Vec::with_capacity(cases.len());

    for case in cases {
        let t0 = Instant::now();
        let routed = router.route(&case.text);
        latency_samples_us.push(t0.elapsed().as_micros());
        let decision = match routed {
            Ok(d) => d,
            Err(e) => {
                eprintln!("  routing error for {:?}: {}", case.text, e);
                wrong += 1;
                *fn_map.entry(case.expected_route.clone()).or_default() += 1;
                continue;
            }
        };

        let top2 = decision
            .candidates
            .iter()
            .take(2)
            .any(|c| c.route == case.expected_route);
        if top2 {
            top2_correct += 1;
        }

        // Clone status to avoid partial-move conflicts when accessing selected_route
        match decision.status.clone() {
            DecisionStatus::BelowThreshold => {
                below_threshold += 1;
                *fn_map.entry(case.expected_route.clone()).or_default() += 1;
            }
            DecisionStatus::Ambiguous => {
                ambiguous += 1;
                *fn_map.entry(case.expected_route.clone()).or_default() += 1;
            }
            _ => match &decision.selected_route {
                Some(got) if got == &case.expected_route => {
                    correct += 1;
                    *tp_map.entry(got.clone()).or_default() += 1;
                }
                Some(got) => {
                    wrong += 1;
                    *fp_map.entry(got.clone()).or_default() += 1;
                    *fn_map.entry(case.expected_route.clone()).or_default() += 1;
                    *confusion
                        .entry((case.expected_route.clone(), got.clone()))
                        .or_default() += 1;
                }
                None => {
                    wrong += 1;
                    *fn_map.entry(case.expected_route.clone()).or_default() += 1;
                }
            },
        }
    }

    let total = cases.len();
    let accuracy = if total > 0 {
        correct as f32 / total as f32
    } else {
        0.0
    };
    let top2_accuracy = if total > 0 {
        top2_correct as f32 / total as f32
    } else {
        0.0
    };

    let all_routes: std::collections::HashSet<String> =
        cases.iter().map(|c| c.expected_route.clone()).collect();

    let mut per_route = HashMap::new();
    for route in &all_routes {
        let tp = *tp_map.get(route).unwrap_or(&0);
        let fp = *fp_map.get(route).unwrap_or(&0);
        let false_neg = *fn_map.get(route).unwrap_or(&0);
        let precision = if tp + fp > 0 {
            tp as f32 / (tp + fp) as f32
        } else {
            0.0
        };
        let recall = if tp + false_neg > 0 {
            tp as f32 / (tp + false_neg) as f32
        } else {
            0.0
        };
        let f1 = if precision + recall > 0.0 {
            2.0 * precision * recall / (precision + recall)
        } else {
            0.0
        };
        per_route.insert(
            route.clone(),
            RouteMetrics {
                tp,
                fp,
                false_neg,
                precision,
                recall,
                f1,
            },
        );
    }

    let mut top_confusion: Vec<ConfusionEntry> = confusion
        .into_iter()
        .map(|((expected, got), count)| ConfusionEntry {
            expected,
            got,
            count,
        })
        .collect();
    top_confusion.sort_by_key(|e| std::cmp::Reverse(e.count));
    top_confusion.truncate(10);

    let latency = compute_latency(&latency_samples_us);

    EvalMetrics {
        total,
        correct,
        wrong,
        ambiguous,
        below_threshold,
        accuracy,
        top2_correct,
        top2_accuracy,
        per_route,
        top_confusion,
        latency,
    }
}

fn compute_latency(samples_us: &[u128]) -> LatencyMetrics {
    if samples_us.is_empty() {
        return LatencyMetrics::default();
    }
    let mut sorted: Vec<u128> = samples_us.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    let pct = |p: f64| -> f64 {
        let idx = ((p / 100.0) * (n as f64 - 1.0)).round() as usize;
        sorted[idx.min(n - 1)] as f64 / 1000.0
    };
    let sum: u128 = sorted.iter().sum();
    LatencyMetrics {
        samples: n,
        mean_ms: (sum as f64 / n as f64) / 1000.0,
        p50_ms: pct(50.0),
        p95_ms: pct(95.0),
        p99_ms: pct(99.0),
        min_ms: sorted[0] as f64 / 1000.0,
        max_ms: sorted[n - 1] as f64 / 1000.0,
    }
}
