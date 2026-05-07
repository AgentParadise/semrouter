//! Contract tests: each fixture under tests/fixtures/<name>/ asserts that
//! EvalSuite meets its declared thresholds. Real fastembed is used.
//!
//! These tests download the fastembed model on first run (~23MB) into
//! .fastembed_cache/ and take ~5s of cold start. Subsequent runs are fast.
//! Run with:
//!
//!     cargo test --test contract --release -- --nocapture

use semrouter::testing::EvalSuite;

#[test]
fn minimal_corpus_meets_thresholds() {
    let report = EvalSuite::from_dir("tests/fixtures/minimal")
        .expect("loading minimal fixture")
        .assert_passes();
    eprintln!(
        "minimal: accuracy={:.3} top2={:.3} p95={:.2}ms load={:.0}ms",
        report.metrics.accuracy,
        report.metrics.top2_accuracy,
        report.metrics.latency.p95_ms,
        report.load_ms
    );
}

#[test]
fn voice_assistant_corpus_meets_thresholds() {
    let report = EvalSuite::from_dir("tests/fixtures/voice-assistant")
        .expect("loading voice-assistant fixture")
        .assert_passes();
    eprintln!(
        "voice-assistant: accuracy={:.3} top2={:.3} p95={:.2}ms load={:.0}ms",
        report.metrics.accuracy,
        report.metrics.top2_accuracy,
        report.metrics.latency.p95_ms,
        report.load_ms
    );
}
