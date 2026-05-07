//! Regression guard: HttpEmbedder is removed. Consumers needing HTTP-backed
//! embeddings implement the public `EmbeddingProvider` trait themselves with
//! their HTTP client of choice (~30 lines, pick ureq for ~5 deps or reqwest
//! for the kitchen sink). If HttpEmbedder is reintroduced in the future, this
//! test forces a deliberate decision (delete the test, justify the dep
//! graph regrowth, update README's "zero default deps" claim).

#[test]
fn no_reqwest_in_cargo_toml() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    assert!(!manifest.contains("reqwest"), "reqwest should not be a dependency");
    assert!(!manifest.contains("tokio"), "tokio should not be a dependency (only reqwest needed it)");
}
