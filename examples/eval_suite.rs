//! Load a fixture, run the eval, assert thresholds, print the report.
//!
//! Run with: `cargo run --example eval_suite --release`
//! Requires `tests/fixtures/minimal/` from the GitHub source tree
//! (excluded from the crates.io tarball).

use semrouter::testing::EvalSuite;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture = "tests/fixtures/minimal";
    println!("Loading fixture from {}", fixture);

    let report = EvalSuite::from_dir(fixture)?.assert_passes();

    println!("\nResults:");
    println!("  accuracy:        {:.3}", report.metrics.accuracy);
    println!("  top-2 accuracy:  {:.3}", report.metrics.top2_accuracy);
    println!("  p50 latency:     {:.2} ms", report.metrics.latency.p50_ms);
    println!("  p95 latency:     {:.2} ms", report.metrics.latency.p95_ms);
    println!("  p99 latency:     {:.2} ms", report.metrics.latency.p99_ms);
    println!("  load time:       {:.0} ms", report.load_ms);

    Ok(())
}
