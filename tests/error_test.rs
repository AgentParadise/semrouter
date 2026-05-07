use semrouter::error::EvalSuiteError;
use semrouter::testing::EvalSuite;

#[test]
fn from_dir_returns_typed_error_when_router_toml_missing() {
    let dir = tempfile::tempdir().unwrap();
    let result = EvalSuite::from_dir(dir.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, EvalSuiteError::ConfigLoad(_)), "got {err:?}");
}

#[test]
fn from_dir_returns_typed_error_when_thresholds_malformed() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("router.toml"),
        concat!(
            "[router]\n",
            "name=\"x\"\nversion=\"0.1.0\"\nembedding_model=\"mock\"\n",
            "vector_dimension=64\ntop_k=1\n",
            "minimum_score=0.01\nminimum_margin=0.001\nfallback_route=\"x\"\n",
            "[storage]\nroutes_file=\"r.jsonl\"\nhard_negatives_file=\"h.jsonl\"\n",
            "feedback_file=\"f.jsonl\"\ndecision_log_file=\"d.jsonl\"\nindex_dir=\"i\"\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.path().join("thresholds.toml"), "this = is = bad").unwrap();

    let result = EvalSuite::from_dir(dir.path());
    assert!(matches!(
        result.unwrap_err(),
        EvalSuiteError::ThresholdsParse(_)
    ));
}
