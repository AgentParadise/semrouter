# semrouter

A lightweight, file-based semantic router for agent/model/workflow dispatch, written in Rust.

Routes user requests to categories, agents, or models by comparing their embeddings to a curated set of labeled examples — no LLM call required in the hot path.

## Concept

```
input text → embed → compare to examples → score routes → apply thresholds → decision
```

Routing behavior improves over time by adding reviewed examples and rebuilding the index. No neural retraining required.

## Quick Start

```bash
# Route a request
semrouter route "Help me debug this Python error"

# List loaded routes
semrouter routes

# Show config and stats
semrouter info
```

## Output Format

```json
{
  "input": "Help me debug this Python error",
  "selected_route": "coding",
  "status": "accepted",
  "confidence": {
    "top_score": 0.542,
    "second_score": 0.18,
    "margin": 0.362
  },
  "candidates": [
    {
      "route": "coding",
      "score": 0.542,
      "matched_examples": ["ex_001", "ex_005", "ex_006"]
    }
  ]
}
```

## Decision Statuses

| Status | Meaning |
|---|---|
| `accepted` | Route selected with sufficient confidence |
| `ambiguous` | Top route score is above threshold but margin is too small |
| `below_threshold` | Top score is below minimum |
| `needs_review` | No examples loaded or unknown state |

## File Layout

```
semrouter/
  router.toml          # Config: thresholds, embedder, storage paths
  routes.jsonl         # Route examples (source of truth)
  hard_negatives.jsonl # Counter-examples to sharpen routing
  eval.jsonl           # Held-out evaluation pairs
```

## router.toml

```toml
[router]
name = "semrouter"
embedding_model = "fastembed"  # or "mock" for testing, "http" for OpenAI-compatible
top_k = 3                      # Average top-N similarities per route
minimum_score = 0.22           # Tuned for fastembed (all-MiniLM-L6-v2)
minimum_margin = 0.005         # Min gap between top and second route score
fallback_route = "needs_review"
```

## routes.jsonl Format

Each line is one labeled example:

```jsonl
{"id":"ex_001","route":"coding","text":"Help me debug this Python error","tags":["debugging","python"]}
{"id":"ex_002","route":"second_brain_capture","text":"Save this idea to my knowledge base","tags":["knowledge"]}
```

## Scoring

**Top-K per route**: For each candidate route, find the top-K examples by cosine similarity, then average their scores. This is more robust than nearest-neighbor alone.

**Margin check**: If the gap between the first and second route score is below `minimum_margin`, the decision is `ambiguous` even if the top score passes `minimum_score`.

**Hard negatives**: Counter-examples in `hard_negatives.jsonl` apply a penalty to routes whose examples are too similar to things they should NOT match.

## Embedding Notes

- **Mock embedder** (`--embedder mock`): 64-dim keyword-bag vectors. Deterministic, fast, no download. Good for testing routing logic. Score range ~0.25–0.60. Thresholds: `minimum_score = 0.25`, `minimum_margin = 0.04`.
- **FastEmbed** (`--embedder fastembed`): Local ONNX `all-MiniLM-L6-v2` via `fastembed` crate, 384-dim. Downloads ~23MB model to `.fastembed_cache/` on first use. Score range ~0.22–0.62. Thresholds: `minimum_score = 0.22`, `minimum_margin = 0.005`.
- **HTTP** (`--embedder http`): OpenAI-compatible `/v1/embeddings`. Endpoint from `[embedding].endpoint` in `router.toml` or `OPENAI_BASE_URL` env var.

Thresholds are tuned per embedder. Changing the embedder usually requires re-tuning `minimum_score` / `minimum_margin` and re-running `eval`.

## Using as a Library

`semrouter` is published as a Rust crate. From a Gitea-hosted consumer, pin by tag:

```toml
[dependencies]
semrouter = { git = "https://github.com/AgentParadise/semrouter", tag = "v0.1.0" }
```

Or, if you've vendored the crate as a git submodule:

```toml
[dependencies]
semrouter = { path = "../vendor/semrouter" }
```

### Routing

```rust
use semrouter::{SemanticRouter, config::RouterConfig, embedding::FastEmbedEmbedder};

let config = RouterConfig::load("router.toml".as_ref())?;
let embedder = Box::new(FastEmbedEmbedder::new()?);
let router = SemanticRouter::load(config, "routes.jsonl".as_ref(), embedder)?;
let decision = router.route("what time is it")?;

if let Some(route) = decision.selected_route {
    // dispatch the route — semrouter is a pure classifier; risk gating
    // and confirmation prompts live in your application layer.
}
```

semrouter returns the route + scores + confidence. Risk classification and confirmation gating are intentionally NOT semrouter's concern — they belong in the consumer's plugin / capability layer where they can actually act on the dispatch.

## Contract Testing

Each consumer keeps its own route corpus + thresholds and asserts quality in its own test suite:

```
my-service/
  tests/
    semrouter_corpus/
      routes.jsonl
      eval.jsonl
      router.toml
      thresholds.toml
    semrouter_corpus.rs
```

```rust
// my-service/tests/semrouter_corpus.rs
use semrouter::testing::EvalSuite;

#[test]
fn route_corpus_meets_quality_bar() {
    EvalSuite::from_dir("tests/semrouter_corpus")
        .unwrap()
        .assert_passes();
}
```

`thresholds.toml` keys (all optional — only set keys are enforced):

```toml
min_accuracy        = 0.85
min_top2_accuracy   = 0.90
min_per_route_f1    = 0.50
max_p50_ms          = 10.0
max_p95_ms          = 25.0
max_p99_ms          = 50.0
max_load_ms         = 15000.0
```

semrouter itself ships two reference fixtures under `tests/fixtures/` that exercise this same machinery — see `tests/contract.rs`.

### CI Gating Without Rust

If your CI doesn't run Rust tests (Python service, shell scripts, etc.), use the CLI:

```bash
semrouter --config router.toml --routes routes.jsonl --embedder fastembed \
    eval --eval-file eval.jsonl --thresholds thresholds.toml
# exit 0 = passed, 1 = threshold breached, 2 = config error
```

## Common Workflows (justfile)

```bash
just              # list all recipes
just build        # cargo build --release
just test         # cargo test (unit + integration; no fastembed download)
just contract     # cargo test --test contract --release (real fastembed)
just eval voice-assistant      # eval against a fixture
just eval-gated voice-assistant # eval + apply thresholds
just route "what time is it"   # one-off route lookup
just ci           # fmt + clippy + test + contract (pre-commit gate)
```

## Versioning

semrouter follows semver with a 0.x convention:

- **0.x.y** (current): pre-1.0; breaking changes can land on minor bumps. The public API surface (`SemanticRouter`, `RouteDecision`, `semrouter::testing::*`) is unstable until 1.0.0.
- **1.0.0** (future): public API frozen. Breaking changes only on major bumps.

Consumers should pin by tag (`tag = "v0.1.0"`), not by branch. Pinning by SHA is also fine for internal use.

Each release tag's commit message includes a `BREAKING CHANGES:` block when applicable. Check `git log v0.X.Y..v0.Y.Z` for migration notes.

## POC Progress

| POC | Phase | Status |
|---|---|---|
| [POC-001](pocs/POC-001/POC-001.md) | Minimal brute-force router | Done |
| [POC-002](pocs/POC-002/) | HTTP embedder, eval framework, experiment runner | Done |
| [POC-003](pocs/POC-003/) | Local ONNX embeddings (fastembed all-MiniLM-L6-v2) | Done |
| [POC-004](pocs/POC-004/) | Expanded examples + hard negatives | Done |

## Implementation Phases

- **Phase 1** (POC-001): Brute-force routing, mock embedder, CLI ✅
- **Phase 2** (POC-002): `eval` command, experiment runner, HTTP embedder ✅
- **Phase 3**: Decision logging, feedback command, promote-feedback
- **Phase 4** (POC-003/004): Local embedder (fastembed-rs / ONNX), hard negatives ✅
- **Phase 5**: Axum HTTP service (not in scope for v0.1.0)
- **Phase 6**: Eval dashboard
