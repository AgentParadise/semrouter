use semrouter::testing::{EvalSuite, Thresholds};
use std::io::Write;
use tempfile::TempDir;

// MockEmbedder carve-out: this test exercises EvalSuite's wiring (file loading,
// threshold parsing, pass/fail logic). Real fastembed is used by contract-test
// fixtures in tests/contract.rs. See CLAUDE.md "Testing principles".

fn write_file(dir: &std::path::Path, name: &str, content: &str) {
    let mut f = std::fs::File::create(dir.join(name)).unwrap();
    f.write_all(content.as_bytes()).unwrap();
}

fn make_fixture_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    write_file(
        dir.path(),
        "routes.jsonl",
        concat!(
        "{\"id\":\"r1\",\"route\":\"alpha\",\"text\":\"alpha one\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r2\",\"route\":\"alpha\",\"text\":\"alpha two\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r3\",\"route\":\"beta\",\"text\":\"beta one\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r4\",\"route\":\"beta\",\"text\":\"beta two\",\"tags\":[],\"risk\":\"low\"}\n",
    ),
    );
    write_file(
        dir.path(),
        "eval.jsonl",
        concat!(
            "{\"text\":\"alpha one\",\"expected_route\":\"alpha\"}\n",
            "{\"text\":\"beta one\",\"expected_route\":\"beta\"}\n",
        ),
    );
    write_file(dir.path(), "router.toml", concat!(
        "[router]\n",
        "name = \"test\"\nversion = \"0.1.0\"\n",
        "embedding_model = \"mock\"\nvector_dimension = 64\n",
        "top_k = 1\nminimum_score = 0.01\nminimum_margin = 0.001\n",
        "fallback_route = \"needs_review\"\n",
        "[storage]\nroutes_file=\"routes.jsonl\"\nhard_negatives_file=\"hard_negatives.jsonl\"\n",
        "feedback_file=\"feedback.jsonl\"\ndecision_log_file=\"decisions.jsonl\"\nindex_dir=\"index\"\n",
    ));
    write_file(
        dir.path(),
        "thresholds.toml",
        concat!("min_accuracy = 0.5\n", "max_p95_ms = 1000.0\n",),
    );
    dir
}

#[test]
fn eval_suite_passes_when_thresholds_met() {
    let dir = make_fixture_dir();
    let suite = EvalSuite::from_dir(dir.path()).unwrap();
    let report = suite.evaluate().unwrap();
    assert!(report.metrics.accuracy >= 0.5);
    assert!(report.metrics.latency.p95_ms < 1000.0);
}

#[test]
fn eval_suite_fails_when_accuracy_below_floor() {
    let dir = make_fixture_dir();
    // 1.01 is impossible to reach (accuracy is capped at 1.0), so this threshold
    // always fires regardless of how well the MockEmbedder routes.
    write_file(dir.path(), "thresholds.toml", "min_accuracy = 1.01\n");
    let suite = EvalSuite::from_dir(dir.path()).unwrap();
    let result = suite.evaluate();
    assert!(
        result.is_err(),
        "expected threshold failure, got {:?}",
        result.is_ok()
    );
    let failures = result.unwrap_err().failures;
    assert!(failures.iter().any(|f| f.contains("accuracy")));
}

#[test]
fn thresholds_load_only_set_keys() {
    let t: Thresholds = toml::from_str("min_accuracy = 0.9\n").unwrap();
    assert_eq!(t.min_accuracy, Some(0.9));
    assert_eq!(t.max_p95_ms, None);
}

#[test]
fn eval_suite_passes_when_thresholds_file_is_empty() {
    let dir = make_fixture_dir();
    // Empty thresholds.toml is a valid TOML doc → all-None Thresholds → no gates.
    write_file(dir.path(), "thresholds.toml", "");
    let suite = EvalSuite::from_dir(dir.path()).unwrap();
    let report = suite
        .evaluate()
        .expect("empty thresholds should not gate anything");
    assert!(report.metrics.total > 0);
}
