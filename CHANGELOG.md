# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
with a 0.x convention: breaking changes can land on minor bumps until 1.0.0.

## [Unreleased]

### Changed (BREAKING: pre-publication)
- **Default features flipped to `[]`** (was `["fastembed", "cli"]`).
  `cargo add semrouter` now produces a lean library (~23 transitive crates) with
  no embedder bundled. Consumers explicitly opt into `fastembed` or implement
  their own `EmbeddingProvider`. CLI install requires `--features cli,fastembed`.
- This makes "use semrouter without fastembed and avoid its transitive deps"
  the default path of least resistance, matching the project's zero-default-deps
  goal (ADR-0001).

### Removed (BREAKING: pre-publication cleanup)
- `MockEmbedder` removed from public API entirely. It was a 64-dim keyword-bag
  helper for testing routing math, not a real embedder, and its presence in the
  public surface invited misuse (e.g. accidental CLI default, misleading first
  impressions). Lifted to `tests/common/test_embedder.rs` as `BagOfWordsEmbedder`,
  visible only to integration tests.
- CLI `--embedder mock` flag removed; default is now `--embedder fastembed`.
- `EvalSuite`'s `embedding_model = "mock"` config string no longer accepted.

### Added
- `EvalSuite::from_dir_with_embedder(path, Box<dyn EmbeddingProvider>)` for
  tests that need to inject a fast deterministic embedder without the fastembed
  model download.

## [0.1.1] - 2026-05-07

First public release on crates.io. Slimmed dep graph (254 → ~21 lean / ~210 default), polished public API.

### Removed (BREAKING)
- **`HttpEmbedder` removed entirely.** Pulled in `reqwest` + `tokio` for an
  OpenAI-API HTTP client that contradicted the project's "no network in the
  hot path" pitch. Consumers needing HTTP-backed embeddings implement the
  public `EmbeddingProvider` trait themselves (~30 lines, see README for a
  `ureq`-based example).
- `--embedder http` CLI flag removed.
- `EmbedderType::Http` variant removed.
- `reqwest`, `tokio`, `anyhow`, `chrono` removed from `[dependencies]`.

### Changed (BREAKING)
- Renamed crate `semroute` → `semrouter`.
- `EvalSuite::from_dir` now returns `Result<Self, EvalSuiteError>` instead of
  `Result<Self, String>`. Match on `ConfigLoad` / `ThresholdsRead` /
  `ThresholdsParse` for typed handling.
- `fastembed` is now an opt-in feature (default-on). `default-features = false`
  drops fastembed from your dep graph.
- `clap` is now behind the `cli` feature (default-on). Library-only consumers
  build without it.
- The `semrouter` binary requires the `cli` and `fastembed` features.

### Added
- `LICENSE` file (MIT).
- Crates.io publish metadata in `Cargo.toml`.
- `examples/quickstart.rs` and `examples/eval_suite.rs`.
- `#![warn(missing_docs)]` lint at lib.rs; doc comments on every public item.
- GitHub Actions CI (fmt + clippy + test + doc + examples + dep-features matrix
  with lean-build regression guard).
- README rewrite with badges, install snippets for both feature profiles,
  30-second example, BYO-embedder example, decision JSON, contract testing,
  CLI, real performance numbers, status, roadmap.
- `CONTRIBUTING.md` and issue/PR templates.
- `src/time_util.rs`: std-only ISO-8601 + compact timestamp formatters,
  replacing chrono. Hinnant's `civil_from_days` algorithm.

### Migration from 0.1.0 (internal preview)
- Update Cargo.toml: `semroute = "0.1"` → `semrouter = "0.1"`.
- Update imports: `use semroute::...` → `use semrouter::...`.
- If you used `HttpEmbedder`, port to a custom `EmbeddingProvider` impl with
  your HTTP client of choice. The README has a `ureq` example.
- If you matched on `EvalSuite::from_dir` errors as `String`, switch to
  matching on `EvalSuiteError`.

### Dependency footprint
| Profile | Transitive crates |
|---|---|
| `default-features = false` | ~21 |
| Default (`fastembed` + `cli`) | ~212 |
| v0.1.0 (pre-slim) | 254 |

## [0.1.0] - 2026-05-06

Internal preview, never published to crates.io. The first public-facing
release is 0.1.1.

### Added
- `SemanticRouter` library API.
- File-based config (`router.toml`), routes (`routes.jsonl`), hard negatives,
  eval cases.
- Three embedder backends: `MockEmbedder`, `FastEmbedEmbedder`, `HttpEmbedder`
  (the last removed in 0.1.1).
- Top-K-per-route averaging with hard-negative penalty.
- `semrouter eval` CLI with accuracy / top-2 / per-route F1 / confusion / latency.
- `--thresholds <path>` CLI flag (exit 0 / 1 / 2).
- Public `semrouter::testing` module (`EvalSuite`, `Thresholds`, `EvalReport`).
- Two reference fixtures (`minimal`, `voice-assistant`) + contract tests.
- `justfile` with build/test/eval/contract/ci recipes.

### Removed (vs internal pre-releases)
- Risk policy and `RouteDecision.policy` field. semrouter is a pure classifier.
- `DecisionStatus::RequiresConfirmation`.
- `PolicySection` config block.

[Unreleased]: https://github.com/AgentParadise/semrouter/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/AgentParadise/semrouter/releases/tag/v0.1.1
[0.1.0]: https://github.com/AgentParadise/semrouter/releases/tag/v0.1.0
