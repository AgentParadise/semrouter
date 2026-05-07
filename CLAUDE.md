# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`semrouter` is a file-based semantic router (Rust) that dispatches input text to a route (agent/model/workflow) by comparing embeddings against a curated set of labeled examples in `routes.jsonl`. No LLM is called in the hot path.

Pipeline: `input → embed → cosine vs. examples → top-K per route avg → threshold + margin → decision`.

## Commands

```bash
# Build
cargo build              # debug
cargo build --release

# Tests
cargo test                              # all tests
cargo test --test integration           # one integration file
cargo test --test routing_test
cargo test <name>                       # filter by test name

# Run the CLI (mock embedder by default — no network/model download)
cargo run -- route "Help me debug this Python error"
cargo run -- routes
cargo run -- info
cargo run -- eval --eval-file eval.jsonl --format json --save-experiment

# Switch embedders
cargo run -- --embedder mock      route "..."
cargo run -- --embedder fastembed route "..."   # local ONNX (all-MiniLM-L6-v2), downloads to .fastembed_cache
cargo run -- --embedder http      route "..."   # OpenAI-compatible; needs [embedding] endpoint or OPENAI_BASE_URL
```

The CLI exits non-zero if `routes.jsonl` is missing, the eval file is missing, or the embedder fails to construct.

## Architecture

Single binary + library crate. `src/lib.rs` exposes `SemanticRouter`; `src/main.rs` is the clap CLI.

**Data flow on `route()`** (`src/lib.rs`):
1. `storage::load_examples` reads `routes.jsonl`; `embed_examples` produces `EmbeddedExample`s. Same for `hard_negatives.jsonl` → `EmbeddedHardNegative`.
2. `embedding::EmbeddingProvider` (trait) embeds the input; `normalize` makes cosine = dot product.
3. `scoring::score_routes` groups example similarities by route, averages the top-K, then subtracts a penalty for nearby hard negatives (`hard_negative_penalty` × max sim to any hard-negative).
4. `decision::make_decision` applies `minimum_score` and `minimum_margin` from config and emits a `RouteDecision` with status (`accepted` / `ambiguous` / `below_threshold` / `needs_review`) and candidate scores. semrouter is a pure classifier — risk assessment and confirmation gating belong in the consumer's plugin layer, not here.

**Embedders** (`src/embedding.rs`):
- `MockEmbedder` — 64-dim keyword-bag, deterministic, no network. Used by tests and as CLI default. Score range ~0.25–0.60.
- `FastEmbedEmbedder` — local ONNX `AllMiniLML6V2` via `fastembed` crate, 384-dim. Caches model under `.fastembed_cache/`. Score range ~0.22–0.62.
- `HttpEmbedder` — OpenAI-compatible `/v1/embeddings`. Endpoint from `[embedding].endpoint` in `router.toml` or `OPENAI_BASE_URL` env var.

Thresholds in `router.toml` are tuned **per embedder**. The committed values target `fastembed` (`minimum_score = 0.22`, `minimum_margin = 0.005`); for `mock` use ~0.25 / 0.04. Changing embedder usually means re-tuning thresholds and re-running `eval`.

**Eval / experiments** (`src/eval.rs`, `src/experiment.rs`): `eval` command computes accuracy, top-2 accuracy, per-route precision/recall/F1, and confusion pairs against `eval.jsonl`. `--save-experiment` writes a timestamped JSON snapshot (config + metrics + embedder label) into `experiments/` for cross-run comparison.

## Files at the root

| File | Purpose |
|---|---|
| `router.toml` | Thresholds, embedder config, storage paths |
| `routes.jsonl` | Labeled examples (source of truth for routing) |
| `hard_negatives.jsonl` | Counter-examples — penalize routes that match these |
| `eval.jsonl` | Held-out `{text, expected_route}` pairs for `cargo run -- eval` |
| `experiments/` | Saved eval runs (gitted) |
| `pocs/POC-NNN/` | Phase-by-phase proof-of-concept writeups |

There is no `feedback.jsonl` / `decisions.jsonl` / `index/` yet — those are reserved for later phases (see README "Implementation Phases").

## Testing principles

- **No mocks unless absolutely necessary.** "Necessary" = a real dependency literally cannot run in the test environment (paid API behind a key, hardware that isn't there). Slowness, model downloads, or "it's just a unit test" are not sufficient reasons. Fixtures and contract tests use the real `fastembed` embedder so the latency and accuracy numbers reflect production behavior. The `MockEmbedder` exists to validate routing logic in isolation, not as a default test substitute.
- **Numbers must be trustworthy.** If a benchmark or eval result would change shape under real dependencies, the test is lying. Prefer a slower honest test over a fast misleading one.

## Conventions

- Errors flow through `RouterError` (`src/error.rs`); `main.rs` prints and `exit(1)`s on each command boundary.
- All vectors are unit-normalized before scoring. If you add a new embedder, normalize in the provider or rely on the `normalize()` call in `SemanticRouter::route`.
- `storage::load_examples` skips blank lines and surfaces parse errors with line numbers — keep that behavior when extending JSONL formats.
