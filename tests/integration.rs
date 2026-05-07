use semrouter::eval::{load_eval_cases, run_eval};
use semrouter::{config::RouterConfig, embedding::MockEmbedder, SemanticRouter};
use std::io::Write;
use tempfile::NamedTempFile;

fn write_temp(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

fn make_router_with_thresholds(
    routes_file: &std::path::Path,
    min_score: f32,
    min_margin: f32,
) -> SemanticRouter {
    let mut config = RouterConfig::default_config();
    config.router.minimum_score = min_score;
    config.router.minimum_margin = min_margin;
    SemanticRouter::load(config, routes_file, Box::new(MockEmbedder::new())).unwrap()
}

#[test]
fn eval_load_and_run_roundtrip() {
    let routes = write_temp(concat!(
        "{\"id\":\"r1\",\"route\":\"coding\",\"text\":\"debug python error\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r2\",\"route\":\"coding\",\"text\":\"fix rust compile error\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r3\",\"route\":\"coding\",\"text\":\"write a unit test for this function\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r4\",\"route\":\"research\",\"text\":\"find information about this topic\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r5\",\"route\":\"research\",\"text\":\"look up latest research papers\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"r6\",\"route\":\"research\",\"text\":\"search web for recent articles\",\"tags\":[],\"risk\":\"low\"}\n",
    ));

    let eval_cases = write_temp(concat!(
        "{\"text\":\"help me debug this python code error\",\"expected_route\":\"coding\"}\n",
        "{\"text\":\"find recent research papers on machine learning\",\"expected_route\":\"research\"}\n",
        "{\"text\":\"write unit tests for this function\",\"expected_route\":\"coding\"}\n",
    ));

    // Use low thresholds so mock embedder can produce decisions
    let router = make_router_with_thresholds(routes.path(), 0.15, 0.01);

    let cases = load_eval_cases(eval_cases.path()).unwrap();
    assert_eq!(cases.len(), 3);

    let metrics = run_eval(&router, &cases);

    assert_eq!(metrics.total, 3);
    assert_eq!(
        metrics.correct + metrics.wrong + metrics.ambiguous + metrics.below_threshold,
        metrics.total,
        "outcome counts must partition total"
    );
    assert!(metrics.accuracy >= 0.0 && metrics.accuracy <= 1.0);
    assert!(metrics.top2_accuracy >= metrics.accuracy, "top2 >= top1");
    assert!(metrics.per_route.contains_key("coding"));
    assert!(metrics.per_route.contains_key("research"));
}

#[test]
fn eval_load_skips_blank_and_comment_lines() {
    let eval_cases = write_temp(concat!(
        "\n",
        "// this is a comment\n",
        "{\"text\":\"debug python\",\"expected_route\":\"coding\"}\n",
        "\n",
    ));
    let cases = load_eval_cases(eval_cases.path()).unwrap();
    assert_eq!(cases.len(), 1);
}

#[test]
fn eval_metrics_accuracy_is_zero_when_all_wrong() {
    // Only one route in index but eval expects a different route
    let routes = write_temp(concat!(
        "{\"id\":\"x1\",\"route\":\"coding\",\"text\":\"debug python error\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"x2\",\"route\":\"coding\",\"text\":\"fix rust code bug\",\"tags\":[],\"risk\":\"low\"}\n",
        "{\"id\":\"x3\",\"route\":\"coding\",\"text\":\"write unit test in python\",\"tags\":[],\"risk\":\"low\"}\n",
    ));
    let eval_cases = write_temp(
        "{\"text\":\"help me plan my product strategy\",\"expected_route\":\"strategy_planning\"}\n"
    );

    let router = make_router_with_thresholds(routes.path(), 0.01, 0.001);
    let cases = load_eval_cases(eval_cases.path()).unwrap();
    let metrics = run_eval(&router, &cases);

    assert_eq!(metrics.total, 1);
    // The only route in the index is "coding", so expected "strategy_planning" won't match
    assert_eq!(metrics.correct, 0);
}
