use semrouter::eval::{load_eval_cases, run_eval};
use semrouter::{config::RouterConfig, embedding::MockEmbedder, SemanticRouter};
use std::io::Write;
use tempfile::NamedTempFile;

fn write_temp(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

// MockEmbedder carve-out: this test validates the structural wiring of the latency
// pipeline (samples collected, ordering invariants, struct populated). Real fastembed
// is reserved for the contract-test fixtures in tests/contract.rs, which assert real
// latency floors. See CLAUDE.md "Testing principles" — using a real embedder here
// would add a ~23MB model download to a fast machinery test for zero added signal.
#[test]
fn run_eval_records_latency_metrics() {
    let routes = write_temp(concat!(
        "{\"id\":\"r1\",\"route\":\"a\",\"text\":\"alpha\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r2\",\"route\":\"a\",\"text\":\"alpha bravo\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r3\",\"route\":\"b\",\"text\":\"beta\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r4\",\"route\":\"b\",\"text\":\"beta gamma\",\"tags\":[],\"risk\":\"low\"}\n",
    ));
    let cases = write_temp(concat!(
        "{\"text\":\"alpha\",\"expected_route\":\"a\"}\n",
        "{\"text\":\"beta\",\"expected_route\":\"b\"}\n",
    ));

    let mut config = RouterConfig::default_config();
    config.router.minimum_score = 0.01;
    config.router.minimum_margin = 0.001;
    let router =
        SemanticRouter::load(config, routes.path(), Box::new(MockEmbedder::new())).unwrap();

    let metrics = run_eval(&router, &load_eval_cases(cases.path()).unwrap());

    assert_eq!(metrics.total, 2);
    assert!(metrics.latency.mean_ms >= 0.0);
    assert!(metrics.latency.p50_ms >= 0.0);
    assert!(metrics.latency.p95_ms >= metrics.latency.p50_ms);
    assert!(metrics.latency.p99_ms >= metrics.latency.p95_ms);
    assert_eq!(metrics.latency.samples, 2);
    // Structural invariant: every eval case must contribute exactly one latency sample
    // (including error paths). If this ever drifts, the latency distribution is lying.
    assert_eq!(
        metrics.latency.samples, metrics.total,
        "samples count must equal total cases"
    );
}

#[test]
fn run_eval_with_no_cases_returns_zero_latency() {
    let routes = write_temp(concat!(
        "{\"id\":\"r1\",\"route\":\"a\",\"text\":\"alpha\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r2\",\"route\":\"a\",\"text\":\"alpha bravo\",\"tags\":[],\"risk\":\"low\"}\n",
    ));

    let mut config = RouterConfig::default_config();
    config.router.minimum_score = 0.01;
    config.router.minimum_margin = 0.001;
    let router =
        SemanticRouter::load(config, routes.path(), Box::new(MockEmbedder::new())).unwrap();

    let metrics = run_eval(&router, &[]);

    assert_eq!(metrics.total, 0);
    assert_eq!(metrics.latency.samples, 0);
    assert_eq!(metrics.latency.mean_ms, 0.0);
    assert_eq!(metrics.latency.p50_ms, 0.0);
    assert_eq!(metrics.latency.p99_ms, 0.0);
    assert_eq!(metrics.latency.min_ms, 0.0);
    assert_eq!(metrics.latency.max_ms, 0.0);
}
