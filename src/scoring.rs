use crate::embedding::cosine_similarity;
use crate::route::{EmbeddedExample, EmbeddedHardNegative};
use std::collections::HashMap;

/// Minimum cosine similarity to a hard negative before the penalty fires.
/// Below this threshold the input is not considered a genuine confusion.
const HN_SIMILARITY_THRESHOLD: f32 = 0.40;

/// A route with its aggregate similarity score after optional hard-negative penalty.
#[derive(Debug, Clone)]
pub struct ScoredCandidate {
    /// The route name.
    pub route: String,
    /// Aggregate score after applying any hard-negative penalty.
    pub score: f32,
    /// IDs of the top-k examples that contributed to this score.
    pub matched_example_ids: Vec<String>,
}

/// Score all routes by averaging the top-k similarities per route.
/// If hard_negatives are provided, applies a proportional penalty when
/// the input is similar to a hard negative for a given route.
pub fn score_routes(
    input_embedding: &[f32],
    examples: &[EmbeddedExample],
    top_k: usize,
    hard_negatives: &[EmbeddedHardNegative],
    penalty: f32,
) -> Vec<ScoredCandidate> {
    // Group similarity scores by route
    let mut route_scores: HashMap<String, Vec<(f32, String)>> = HashMap::new();

    for ex in examples {
        let sim = cosine_similarity(input_embedding, &ex.embedding);
        route_scores
            .entry(ex.example.route.clone())
            .or_default()
            .push((sim, ex.example.id.clone()));
    }

    // For each route, take top-k and average, then apply hard negative penalty
    let mut candidates: Vec<ScoredCandidate> = route_scores
        .into_iter()
        .map(|(route, mut scores)| {
            scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let k = scores.len().min(top_k);
            let top = &scores[..k];
            let avg_score = top.iter().map(|(s, _)| s).sum::<f32>() / k as f32;
            let matched_ids = top.iter().map(|(_, id)| id.clone()).collect();

            // Threshold penalty: apply full penalty only when input is clearly similar to a hard
            // negative (similarity > threshold). Proportional scaling caused collateral damage to
            // genuine queries that share surface vocabulary with the hard negatives.
            let applied_penalty = if !hard_negatives.is_empty() {
                let max_hn_sim = hard_negatives
                    .iter()
                    .filter(|hn| hn.hn.route == route)
                    .map(|hn| cosine_similarity(input_embedding, &hn.embedding))
                    .fold(0.0f32, f32::max);
                if max_hn_sim > HN_SIMILARITY_THRESHOLD {
                    penalty
                } else {
                    0.0
                }
            } else {
                0.0
            };

            ScoredCandidate {
                route,
                score: (avg_score - applied_penalty).max(0.0),
                matched_example_ids: matched_ids,
            }
        })
        .collect();

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::{normalize, EmbeddingProvider};
    use crate::error::RouterError;
    use crate::route::{EmbeddedExample, RiskLevel, RouteExample};

    // Minimal inline keyword-bag embedder for this unit test only.
    // Keeps scoring.rs free of a dependency on the test-only BagOfWordsEmbedder
    // in tests/common/ (which is not reachable from src/ unit tests).
    struct InlineKwEmbed;
    impl EmbeddingProvider for InlineKwEmbed {
        fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
            const KW: &[(&str, usize)] = &[
                ("debug", 0),
                ("code", 1),
                ("error", 2),
                ("fix", 6),
                ("rust", 7),
                ("python", 8),
                ("compile", 11),
                ("test", 4),
                ("save", 16),
                ("brain", 17),
                ("note", 18),
                ("knowledge", 19),
                ("capture", 20),
                ("store", 21),
                ("idea", 22),
                ("insight", 29),
            ];
            let lower = text.to_lowercase();
            let mut v = vec![0.0f32; 64];
            for word in lower.split_whitespace() {
                let word = word.trim_matches(|c: char| !c.is_alphanumeric());
                for &(kw, dim) in KW {
                    if word == kw || word.starts_with(kw) {
                        v[dim] += 1.0;
                    }
                }
            }
            normalize(&mut v);
            Ok(v)
        }
        fn dimension(&self) -> usize {
            64
        }
    }

    fn make_example(id: &str, route: &str, text: &str) -> EmbeddedExample {
        let embedding = InlineKwEmbed.embed(text).unwrap();
        EmbeddedExample {
            example: RouteExample {
                id: id.to_string(),
                route: route.to_string(),
                text: text.to_string(),
                tags: vec![],
                risk: RiskLevel::Low,
            },
            embedding,
        }
    }

    #[test]
    fn coding_input_scores_higher_for_coding_route() {
        let examples = vec![
            make_example("e1", "coding", "Help me debug this Python error"),
            make_example("e2", "coding", "Fix this Rust compile error"),
            make_example("e3", "coding", "Write a unit test for this function"),
            make_example(
                "e4",
                "second_brain_capture",
                "Save this idea to my knowledge base",
            ),
            make_example(
                "e5",
                "second_brain_capture",
                "Store this thought in my brain",
            ),
            make_example(
                "e6",
                "second_brain_capture",
                "Capture this insight in my notes",
            ),
        ];

        let input = InlineKwEmbed
            .embed("debug this code error in python")
            .unwrap();
        let candidates = score_routes(&input, &examples, 3, &[], 0.0);

        assert!(!candidates.is_empty());
        assert_eq!(
            candidates[0].route,
            "coding",
            "Expected coding to win, got: {:?}",
            candidates
                .iter()
                .map(|c| (&c.route, c.score))
                .collect::<Vec<_>>()
        );
    }
}
