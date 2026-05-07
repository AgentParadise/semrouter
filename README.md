<p align="center">
  <img src="https://raw.githubusercontent.com/AgentParadise/semrouter/main/assets/banner-v1.png" alt="semrouter — Semantic Routing Engine" />
</p>

# semrouter

[![CI](https://github.com/AgentParadise/semrouter/actions/workflows/ci.yml/badge.svg)](https://github.com/AgentParadise/semrouter/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/semrouter.svg)](https://crates.io/crates/semrouter)
[![docs.rs](https://docs.rs/semrouter/badge.svg)](https://docs.rs/semrouter)

A lightweight, file-based semantic router for agent / model / workflow dispatch. Routes input text to a labeled route by comparing embeddings against a curated set of examples. **Zero default dependencies beyond `serde`, `serde_json`, `toml`, and `thiserror`** — bundle a local embedder via the `fastembed` feature, or bring your own. No LLM in the hot path. Sub-millisecond routing.

```
input text  →  embed  →  cosine vs. examples  →  top-K per route  →  threshold + margin  →  decision
```

## Why

If you're building an AI agent, voice assistant, or workflow system, you need to dispatch user input to one of N specialized handlers. The naive options — keyword matching (brittle), LLM classifier (slow, expensive, cloud round-trip) — both have real costs. semrouter splits the difference: a tiny local embedding model gives you semantic understanding, and a flat file of labeled examples gives you a router you can edit and version-control.

semrouter is a **pure classifier**. Risk classification, confirmation prompts, and dispatch live in your application — they don't belong in the router. This separation keeps risk policies next to the code that actually runs the dangerous thing.

## Install

**Batteries-included (default — bundles fastembed local embedder):**

```toml
[dependencies]
semrouter = "0.1"
```

**Lean (lib only — bring your own `EmbeddingProvider`):**

```toml
[dependencies]
semrouter = { version = "0.1", default-features = false }
```

The lean profile compiles against ~21 transitive crates. The default profile pulls in `fastembed` for batteries-included local embeddings (~210 crates, dominated by the ONNX runtime).

## 30-second example

`routes.jsonl`:

```jsonl
{"id":"r1","route":"time","text":"what time is it","tags":["time"],"risk":"low"}
{"id":"r2","route":"time","text":"tell me the current time","tags":["time"],"risk":"low"}
{"id":"r3","route":"weather","text":"is it going to rain","tags":["weather"],"risk":"low"}
{"id":"r4","route":"weather","text":"give me the forecast","tags":["weather"],"risk":"low"}
```

`router.toml`:

```toml
[router]
name = "demo"
version = "0.1.0"
embedding_model = "fastembed/AllMiniLML6V2"
vector_dimension = 384
top_k = 3
minimum_score = 0.22
minimum_margin = 0.005
fallback_route = "needs_review"

[storage]
routes_file = "routes.jsonl"
hard_negatives_file = "hard_negatives.jsonl"
feedback_file = "feedback.jsonl"
decision_log_file = "decisions.jsonl"
index_dir = "index"
```

```rust
use semrouter::{SemanticRouter, config::RouterConfig, embedding::FastEmbedEmbedder};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RouterConfig::load(Path::new("router.toml"))?;
    let embedder = Box::new(FastEmbedEmbedder::new()?);
    let router = SemanticRouter::load(config, Path::new("routes.jsonl"), embedder)?;

    let decision = router.route("got the time")?;
    println!("{:#?}", decision.selected_route);
    // → Some("time")
    Ok(())
}
```

See [`examples/quickstart.rs`](examples/quickstart.rs) for a runnable version.

## Bring your own embedder

The `EmbeddingProvider` trait is one method:

```rust
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError>;
}
```

A custom HTTP-backed provider with `ureq` (~5 transitive crates):

```rust
use semrouter::embedding::EmbeddingProvider;
use semrouter::error::RouterError;

struct OpenAIEmbedder { api_key: String }

impl EmbeddingProvider for OpenAIEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
        let resp: serde_json::Value = ureq::post("https://api.openai.com/v1/embeddings")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(serde_json::json!({
                "input": text,
                "model": "text-embedding-3-small"
            }))
            .map_err(|e| RouterError::Embedding(e.to_string()))?
            .into_json()
            .map_err(|e| RouterError::Embedding(e.to_string()))?;

        Ok(resp["data"][0]["embedding"]
            .as_array().unwrap()
            .iter().map(|v| v.as_f64().unwrap() as f32).collect())
    }
}
```

That's the full HTTP-embedder surface. semrouter doesn't ship one because every consumer wants different things from their HTTP client (retry, batching, observability) — pick yours.

## Decision shape

```json
{
  "input": "got the time",
  "selected_route": "time",
  "status": "accepted",
  "confidence": { "top_score": 0.591, "second_score": 0.241, "margin": 0.350 },
  "candidates": [
    { "route": "time", "score": 0.591, "matched_examples": ["r1", "r2"] },
    { "route": "weather", "score": 0.241, "matched_examples": ["r3"] }
  ]
}
```

Status is one of: `accepted`, `ambiguous`, `below_threshold`, `needs_review`.

## Contract testing your route corpus

Each consumer keeps its own corpus + threshold floors and asserts quality in `cargo test`:

```rust
use semrouter::testing::EvalSuite;

#[test]
fn my_corpus_meets_quality_bar() {
    EvalSuite::from_dir("tests/fixtures/voice-assistant")
        .unwrap()
        .assert_passes();
}
```

`tests/fixtures/voice-assistant/thresholds.toml`:

```toml
min_accuracy        = 0.85
min_top2_accuracy   = 0.90
min_per_route_f1    = 0.50
max_p95_ms          = 25.0
max_load_ms         = 15000.0
```

If your corpus regresses (accuracy drops, latency spikes), CI fails. See [`docs/integration-example.md`](docs/integration-example.md) for the full integration story.

## CLI

The `cli` feature (default-on) provides the `semrouter` binary:

```bash
semrouter --config router.toml --routes routes.jsonl --embedder fastembed route "what time is it"
semrouter --config router.toml --routes routes.jsonl --embedder fastembed eval --eval-file eval.jsonl
semrouter --config router.toml --routes routes.jsonl --embedder fastembed eval \
    --eval-file eval.jsonl --thresholds thresholds.toml
# → exit 0 = passed, exit 1 = threshold breached, exit 2 = config/parse error
```

## Performance

Real numbers from the bundled `voice-assistant` fixture (6 routes, 35 examples, 22 eval cases) using `fastembed/AllMiniLML6V2` on a M-series Mac:

| Metric | Value |
|---|---|
| Accuracy | 90.9% |
| Top-2 accuracy | 100.0% |
| p50 latency | 1.24 ms |
| p95 latency | 2.82 ms |
| p99 latency | 3.74 ms |
| Cold-start (model load) | ~50-70 ms |

The 9.1% "incorrect" cases are correctly routed to the `direct_llm` fallback intent (where they belong). For the 5 first-class intents, F1 is **1.000 across the board**.

## Status

semrouter is **pre-1.0**. The public API surface is unstable — minor version bumps may include breaking changes. Pin to a specific version (`semrouter = "=0.1.1"`) for exact reproducibility.

`v1.0.0` will freeze the API.

## Roadmap

- **v0.2.0** — Configurable embedder. Pick any fastembed-supported model from `router.toml` (`fastembed/AllMiniLML6V2`, `fastembed/BGESmallENV15`, `fastembed/MiniLML12V2`, etc.) with a tradeoff guide in docs.
- **v0.3.0** — Closed-loop learning. `semrouter tag` (interactive CLI to mark recent decisions correct/wrong) + `semrouter promote` (ingest tagged feedback as new routing examples + run `EvalSuite` to gate regression). The router gets better the more you use it.
- **v1.0.0** — API freeze + crates.io 1.0.

## License

MIT — see [LICENSE](LICENSE).
