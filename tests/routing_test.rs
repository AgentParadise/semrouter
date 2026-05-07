mod common;
use common::test_embedder::BagOfWordsEmbedder;

use semrouter::decision::DecisionStatus;
use semrouter::{config::RouterConfig, SemanticRouter};
use std::io::Write;
use tempfile::NamedTempFile;

fn make_test_routes() -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    let examples = r#"{"id":"t1","route":"coding","text":"Help me debug this Python error","tags":[],"risk":"low"}
{"id":"t2","route":"coding","text":"Fix this Rust compile error","tags":[],"risk":"low"}
{"id":"t3","route":"coding","text":"Write a unit test for this function","tags":[],"risk":"low"}
{"id":"t4","route":"second_brain_capture","text":"Save this idea to my second brain","tags":[],"risk":"low"}
{"id":"t5","route":"second_brain_capture","text":"Store this insight in my knowledge base","tags":[],"risk":"low"}
{"id":"t6","route":"second_brain_capture","text":"Capture this thought in my notes","tags":[],"risk":"low"}
{"id":"t7","route":"research","text":"Research best practices for distributed tracing","tags":[],"risk":"low"}
{"id":"t8","route":"research","text":"Find papers on transformer architectures","tags":[],"risk":"low"}
{"id":"t9","route":"research","text":"Look up the latest information about this library","tags":[],"risk":"low"}
"#;
    f.write_all(examples.as_bytes()).unwrap();
    f
}

fn make_router(routes_file: &std::path::Path) -> SemanticRouter {
    // BagOfWordsEmbedder produces lower scores (keyword-bag, not real semantics), so use lower thresholds
    let mut config = RouterConfig::default_config();
    config.router.minimum_score = 0.20;
    config.router.minimum_margin = 0.03;
    let embedder = Box::new(BagOfWordsEmbedder::new());
    SemanticRouter::load(config, routes_file, embedder).expect("Failed to load router")
}

#[test]
fn routes_coding_input_to_coding() {
    let routes = make_test_routes();
    let router = make_router(routes.path());

    let decision = router.route("debug this Python code error").unwrap();
    assert_eq!(
        decision.selected_route.as_deref(),
        Some("coding"),
        "Expected coding, got: {:?}\nCandidates: {:?}",
        decision.selected_route,
        decision.candidates
    );
}

#[test]
fn routes_brain_input_to_second_brain_capture() {
    let routes = make_test_routes();
    let router = make_router(routes.path());

    let decision = router.route("save this idea to my brain").unwrap();
    assert_eq!(
        decision.selected_route.as_deref(),
        Some("second_brain_capture"),
        "Expected second_brain_capture, got: {:?}\nCandidates: {:?}",
        decision.selected_route,
        decision.candidates
    );
}

#[test]
fn routes_research_input_to_research() {
    let routes = make_test_routes();
    let router = make_router(routes.path());

    let decision = router
        .route("research and find information about this topic")
        .unwrap();
    assert_eq!(
        decision.selected_route.as_deref(),
        Some("research"),
        "Expected research, got: {:?}\nCandidates: {:?}",
        decision.selected_route,
        decision.candidates
    );
}

#[test]
fn decision_has_candidates_sorted_by_score() {
    let routes = make_test_routes();
    let router = make_router(routes.path());

    let decision = router.route("debug this Python code error").unwrap();
    assert!(!decision.candidates.is_empty());
    for i in 1..decision.candidates.len() {
        assert!(
            decision.candidates[i - 1].score >= decision.candidates[i].score,
            "Candidates not sorted by score"
        );
    }
}

#[test]
fn decision_has_valid_confidence_fields() {
    let routes = make_test_routes();
    let router = make_router(routes.path());

    let decision = router.route("debug this Python code error").unwrap();
    assert!(decision.confidence.top_score > 0.0);
    assert!(decision.confidence.second_score.is_some());
    assert!(decision.confidence.margin.is_some());
    let margin = decision.confidence.margin.unwrap();
    let top = decision.confidence.top_score;
    let second = decision.confidence.second_score.unwrap();
    assert!(
        (top - second - margin).abs() < 0.01,
        "margin should be top - second"
    );
}

#[test]
fn high_risk_route_routes_or_falls_below_threshold() {
    let mut f = NamedTempFile::new().unwrap();
    let examples = r#"{"id":"h1","route":"unsafe_or_high_risk","text":"Delete all files in production","tags":[],"risk":"high"}
{"id":"h2","route":"unsafe_or_high_risk","text":"Run this dangerous shell command","tags":[],"risk":"high"}
{"id":"h3","route":"unsafe_or_high_risk","text":"Execute this risky command on the server","tags":[],"risk":"high"}
"#;
    f.write_all(examples.as_bytes()).unwrap();

    let mut config = RouterConfig::default_config();
    config.router.minimum_score = 0.20;
    config.router.minimum_margin = 0.03;
    let embedder = Box::new(BagOfWordsEmbedder::new());
    let router = SemanticRouter::load(config, f.path(), embedder).unwrap();

    let decision = router
        .route("delete all files in production directory")
        .unwrap();
    // semrouter is a pure classifier — no policy blocking. Status is one of the classifier states.
    assert!(
        decision.status == DecisionStatus::Accepted
            || decision.status == DecisionStatus::BelowThreshold
            || decision.status == DecisionStatus::Ambiguous
            || decision.status == DecisionStatus::NeedsReview,
        "Expected a classifier status, got: {:?}",
        decision.status
    );
    assert!(
        !decision.candidates.is_empty(),
        "router should always return candidates for any input with a non-empty corpus"
    );
    let json = serde_json::to_string(&decision).unwrap();
    assert!(
        !json.contains("\"policy\""),
        "RouteDecision JSON must not contain a policy field, got: {json}"
    );
}

#[test]
fn route_decision_no_longer_has_policy_field() {
    fn assert_no_policy_field(d: &semrouter::decision::RouteDecision) {
        let _ = &d.input;
        let _ = &d.selected_route;
        let _ = &d.status;
        let _ = &d.confidence;
        let _ = &d.candidates;
        // No d.policy
    }
    let _ = assert_no_policy_field;
}

#[test]
fn decision_status_no_longer_has_requires_confirmation() {
    use semrouter::decision::DecisionStatus;
    fn label(s: DecisionStatus) -> &'static str {
        match s {
            DecisionStatus::Accepted => "accepted",
            DecisionStatus::Ambiguous => "ambiguous",
            DecisionStatus::BelowThreshold => "below_threshold",
            DecisionStatus::NeedsReview => "needs_review",
        }
    }
    let _ = label;
}
