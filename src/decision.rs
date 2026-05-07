use crate::config::RouterConfig;
use crate::scoring::ScoredCandidate;
use serde::{Deserialize, Serialize};

/// The status of a routing decision.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    /// Top route's score and margin both met thresholds; safe to dispatch.
    Accepted,
    /// Top route's score met threshold but margin to second-place was too small.
    Ambiguous,
    /// Top route's score was below `minimum_score`.
    BelowThreshold,
    /// No examples loaded, or other state where no opinion can be formed.
    NeedsReview,
}

impl std::fmt::Display for DecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self).unwrap();
        write!(f, "{}", s.as_str().unwrap_or("unknown"))
    }
}

/// A single route candidate with its aggregate score and matched example IDs.
#[derive(Debug, Serialize, Clone)]
pub struct CandidateOutput {
    /// The route name.
    pub route: String,
    /// Aggregate similarity score (0.0–1.0) for this route.
    pub score: f32,
    /// IDs of the top-k examples that contributed to this score.
    pub matched_examples: Vec<String>,
}

/// Confidence metrics for the top-scoring route.
#[derive(Debug, Serialize, Clone)]
pub struct ConfidenceOutput {
    /// Similarity score of the top-ranked route.
    pub top_score: f32,
    /// Similarity score of the second-ranked route, if present.
    pub second_score: Option<f32>,
    /// Difference between top and second scores (`top_score - second_score`).
    pub margin: Option<f32>,
}

/// The full output of a routing call.
#[derive(Debug, Serialize, Clone)]
pub struct RouteDecision {
    /// The original input text that was routed.
    pub input: String,
    /// The route name selected by the classifier, if status is `Accepted`.
    pub selected_route: Option<String>,
    /// Status of the decision (Accepted / Ambiguous / BelowThreshold / NeedsReview).
    pub status: DecisionStatus,
    /// Confidence metrics: top score, second score, and margin between them.
    pub confidence: ConfidenceOutput,
    /// All candidate routes ranked by score with their matched example IDs.
    pub candidates: Vec<CandidateOutput>,
}

/// Apply config thresholds to scored candidates and produce a [`RouteDecision`].
pub fn make_decision(
    input: &str,
    candidates: Vec<ScoredCandidate>,
    config: &RouterConfig,
) -> RouteDecision {
    let min_score = config.router.minimum_score;
    let min_margin = config.router.minimum_margin;

    let candidate_outputs: Vec<CandidateOutput> = candidates
        .iter()
        .map(|c| CandidateOutput {
            route: c.route.clone(),
            score: (c.score * 1000.0).round() / 1000.0,
            matched_examples: c.matched_example_ids.clone(),
        })
        .collect();

    if candidates.is_empty() {
        return RouteDecision {
            input: input.to_string(),
            selected_route: None,
            status: DecisionStatus::NeedsReview,
            confidence: ConfidenceOutput {
                top_score: 0.0,
                second_score: None,
                margin: None,
            },
            candidates: vec![],
        };
    }

    let top = &candidates[0];
    let top_score = top.score;
    let second_score = candidates.get(1).map(|c| c.score);
    let margin = second_score.map(|s| top_score - s);

    let status = if top_score < min_score {
        DecisionStatus::BelowThreshold
    } else if margin.is_some_and(|m| m < min_margin) {
        DecisionStatus::Ambiguous
    } else {
        DecisionStatus::Accepted
    };

    let selected = match status {
        DecisionStatus::Accepted => Some(top.route.clone()),
        _ => None,
    };

    RouteDecision {
        input: input.to_string(),
        selected_route: selected,
        status,
        confidence: ConfidenceOutput {
            top_score: (top_score * 1000.0).round() / 1000.0,
            second_score: second_score.map(|s| (s * 1000.0).round() / 1000.0),
            margin: margin.map(|m| (m * 1000.0).round() / 1000.0),
        },
        candidates: candidate_outputs,
    }
}
