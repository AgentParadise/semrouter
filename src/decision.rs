use crate::config::RouterConfig;
use crate::scoring::ScoredCandidate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Accepted,
    Ambiguous,
    BelowThreshold,
    NeedsReview,
}

impl std::fmt::Display for DecisionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self).unwrap();
        write!(f, "{}", s.as_str().unwrap_or("unknown"))
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct CandidateOutput {
    pub route: String,
    pub score: f32,
    pub matched_examples: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConfidenceOutput {
    pub top_score: f32,
    pub second_score: Option<f32>,
    pub margin: Option<f32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RouteDecision {
    pub input: String,
    pub selected_route: Option<String>,
    pub status: DecisionStatus,
    pub confidence: ConfidenceOutput,
    pub candidates: Vec<CandidateOutput>,
}

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
