# ADR-0003: BYO embedder via the EmbeddingProvider trait

**Status:** Accepted (2026-05-07, v0.1.1)

## Context

Pre-v0.1.1, semrouter shipped three embedder backends:

1. `MockEmbedder` — 64-dim keyword-bag, deterministic, zero deps. For testing.
2. `FastEmbedEmbedder` — local ONNX MiniLM via the `fastembed` crate. The recommended production embedder.
3. `HttpEmbedder` — OpenAI-compatible `/v1/embeddings` HTTP client built on `reqwest` + `tokio`.

The HTTP embedder pulled in ~85 transitive crates (the entire async stack) for what is, in its core form, a 30-line HTTP request. Worse, it shipped a half-baked implementation: no connection pooling, no retry/backoff, no rate limiting, no observability hooks. Real HTTP-embedding consumers replace that on day one with their own implementation tuned for their service.

## Decision

**semrouter ships only embedders that don't need network or async runtime.** Currently:

- `MockEmbedder` (always available)
- `FastEmbedEmbedder` (behind `fastembed` feature, default-on)

For any other backend — HTTP API, custom local model, candle-based embedder, GPU-accelerated ort directly — consumers implement the public `EmbeddingProvider` trait:

```rust
pub trait EmbeddingProvider: Send + Sync {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError>;
}
```

That's the entire surface. One method. Sync. Returns a flat `Vec<f32>`. Implementing it with any HTTP client is ~30 lines:

```rust
struct OpenAIEmbedder { api_key: String }

impl EmbeddingProvider for OpenAIEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>, RouterError> {
        let resp: serde_json::Value = ureq::post("https://api.openai.com/v1/embeddings")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(serde_json::json!({"input": text, "model": "text-embedding-3-small"}))
            .map_err(|e| RouterError::Embedding(e.to_string()))?
            .into_json()
            .map_err(|e| RouterError::Embedding(e.to_string()))?;
        Ok(resp["data"][0]["embedding"].as_array().unwrap()
            .iter().map(|v| v.as_f64().unwrap() as f32).collect())
    }
}
```

The full README has this example.

## Consequences

### Positive

- semrouter's default dep tree drops by ~85 crates (no `reqwest`, no `tokio`).
- Consumers pick their own HTTP client (`ureq` for ~5 deps; `reqwest` for full bells and whistles; `hyper` for fanatics; `tokio` if they're already async).
- Consumers control retry, backoff, batching, observability — all the details that vary per service.
- The `EmbeddingProvider` trait is sync, which keeps the library sync. Async creep is contained.

### Negative

- "Quick start with OpenAI" requires writing 30 lines instead of just enabling a feature flag. Documented thoroughly in README and integration-example.md, but it's still friction.
- We don't get the convenience of `semrouter = { features = ["openai"] }`. (We could add such a feature in the future, gated correctly. Out of scope for v0.1.x.)

### Neutral

- A community contribution that ships an `semrouter-openai` or `semrouter-cohere` adapter crate would be welcome and is the natural Rust ecosystem pattern. Out of scope for v0.1.x.

## References

- CHANGELOG.md v0.1.1 (HttpEmbedder removal)
- `src/embedding.rs` — `EmbeddingProvider` trait definition
- README.md "Bring your own embedder" section
