# ADR-0001: Zero default dependencies as a non-negotiable goal

**Status:** Accepted (2026-05-07, v0.1.1)

## Context

Rust crates that target the AI/agent ecosystem often pull in 200+ transitive dependencies — async runtimes, HTTP clients, JSON/YAML/TOML parsers, ML frameworks, etc. Each transitive dep is a build-time cost (cold builds, CI minutes), a security surface (supply-chain attacks, CVEs to track), and a friction point for downstream consumers (especially those targeting embedded, WASM, or constrained CI environments).

semrouter is a small library — at its core, it does:
1. Read JSONL + TOML files
2. Compute cosine similarity between embedding vectors
3. Apply thresholds and emit a decision struct

None of that needs an async runtime, HTTP client, or full ML framework. The only "fat" dependency is the embedding model itself (fastembed), which is genuinely needed for batteries-included use cases.

The pre-v0.1.1 dep tree was 254 transitive crates, dominated by fastembed (190), reqwest+tokio (~85 overlapping), clap (17), and a few small additions. Most of that is unnecessary for a typical consumer.

## Decision

**The default dep tree must stay minimal — under 25 transitive crates for `default-features = false` builds.** Every dependency must justify its existence:

1. **Mandatory deps** (`serde`, `serde_json`, `toml`, `thiserror`): the data model is JSONL + TOML, and consumers need typed errors. ~12 transitive crates total.
2. **Optional deps behind feature flags** (`fastembed`, `clap`): batteries-included for users who want them, opt-out for users who don't.
3. **No dep is added to defaults without an explicit ADR or PR justification.**

CI enforces this with a regression guard: the `lean-build` job runs `cargo tree --no-default-features` and fails if the count exceeds 25.

### Things explicitly rejected

- **`anyhow`** in the library. Public APIs deserve typed errors (`thiserror::Error` enums). `anyhow` is fine in binaries; not in library code.
- **`chrono`** for timestamps. `std::time::SystemTime` plus a 60-line `civil_from_days` helper covers our needs (ISO-8601 + compact filename formats).
- **`reqwest` / `tokio`** for HTTP embedding. semrouter is not async; its hot path is dot-product math. A user wanting an HTTP-backed embedder implements the public `EmbeddingProvider` trait themselves — the surface is one method, easy to roll with `ureq` (~5 deps) or whatever client they prefer.
- **`async_trait`** (entire ecosystem). `EmbeddingProvider::embed` is sync. Async-in-sync via `tokio::runtime::Runtime::new().block_on(...)` is a smell; we removed it.

## Consequences

### Positive

- A consumer doing `semrouter = { default-features = false }` compiles against ~21 crates. Cold build is seconds, not minutes.
- WASM and embedded targets have a fighting chance.
- CVE surface is small; security review is tractable.
- The "what does this library actually need to do its job?" question stays sharp. New deps require thought.

### Negative

- Some convenience features (HTTP embedder, structured logging, observability hooks) are not bundled. Users must implement them via traits.
- The `EmbeddingProvider` trait must stay sync-only, which constrains future API evolution.
- We don't get the network effects of being part of a popular framework's dep graph.

### Neutral

- `fastembed` (default-on, ~190 transitive deps) is the elephant in the room. It's there because most users want batteries-included local embeddings, but consumers who bring their own embedder pay zero cost for it via `default-features = false`. This is the only acceptable form of "fat" dep — opt-out, never opt-in-required.

## References

- CHANGELOG.md v0.1.1 (the rename + slim release)
- `Cargo.toml` `[features]` block
- `.github/workflows/ci.yml` `lean-build` job (the enforcement mechanism)
