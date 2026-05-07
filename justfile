# semroute — common dev recipes. Run `just` for the list.

default:
    @just --list

# Build release binary
build:
    cargo build --release

# Run all unit + integration tests (no fastembed model download)
test:
    cargo test

# Run contract tests against tests/fixtures/* — uses real fastembed
# (downloads ~23MB model on first run into .fastembed_cache/)
contract:
    cargo test --test contract --release -- --nocapture

# Run all tests including the fastembed-backed contract tests
test-all: test contract

# Run eval against a fixture directory; pass FIXTURE=<name> to pick (default: minimal)
eval FIXTURE="minimal":
    cargo run --release -- \
        --config tests/fixtures/{{FIXTURE}}/router.toml \
        --routes tests/fixtures/{{FIXTURE}}/routes.jsonl \
        --embedder fastembed \
        eval --eval-file tests/fixtures/{{FIXTURE}}/eval.jsonl

# Run eval AND apply thresholds (exits non-zero on regression)
eval-gated FIXTURE="minimal":
    cargo run --release -- \
        --config tests/fixtures/{{FIXTURE}}/router.toml \
        --routes tests/fixtures/{{FIXTURE}}/routes.jsonl \
        --embedder fastembed \
        eval --eval-file tests/fixtures/{{FIXTURE}}/eval.jsonl \
              --thresholds tests/fixtures/{{FIXTURE}}/thresholds.toml

# Try a one-off route against a fixture (FIXTURE defaults to voice-assistant)
route INPUT FIXTURE="voice-assistant":
    cargo run --release -- \
        --config tests/fixtures/{{FIXTURE}}/router.toml \
        --routes tests/fixtures/{{FIXTURE}}/routes.jsonl \
        --embedder fastembed \
        route "{{INPUT}}"

# Save an eval run as a timestamped experiment under experiments/
experiment FIXTURE="voice-assistant":
    cargo run --release -- \
        --config tests/fixtures/{{FIXTURE}}/router.toml \
        --routes tests/fixtures/{{FIXTURE}}/routes.jsonl \
        --embedder fastembed \
        eval --eval-file tests/fixtures/{{FIXTURE}}/eval.jsonl \
              --save-experiment

# Lint
clippy:
    cargo clippy --all-targets -- -D warnings

# Format
fmt:
    cargo fmt --all

# Pre-commit gate: format check + clippy + tests + contract
ci: fmt clippy test contract
