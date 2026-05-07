# Contributing to semrouter

Thanks for your interest! Bug reports, feature requests, and PRs all welcome.

## Quick start

```bash
git clone https://github.com/AgentParadise/semrouter.git
cd semrouter
cargo build --release          # default features (fastembed + cli)
cargo test                     # unit + integration tests
cargo test --test contract --release  # contract tests, downloads ~23MB model on first run
just                           # list dev recipes
```

The `fastembed` crate downloads an ONNX model into `.fastembed_cache/` on first use. Gitignored.

## Build profiles

```bash
cargo build --no-default-features                       # lib only, ~21 crates
cargo build --no-default-features --features fastembed  # lib + fastembed, no CLI
cargo build --release                                   # default = fastembed + cli (full binary)
```

When adding a new feature, decide: does this belong in default features or behind a flag? If it pulls in a new dep stack, default to feature-gating it. The `lean-build` CI job will fail if the no-default-features dep tree grows above 25 crates.

## Testing principles

- **No mocks unless absolutely necessary.** Real dependencies in tests so the numbers are honest. `MockEmbedder` exists to validate routing logic in isolation, not as a default substitute.
- **Contract tests use real fastembed.** Fixtures under `tests/fixtures/<name>/` declare their own `thresholds.toml`. Recalibrate when changing the corpus.
- **Latency numbers must reflect production.** Don't measure under MockEmbedder.

## Pre-commit gate

```bash
just ci  # fmt + clippy -D warnings + test + contract
```

CI runs the same checks plus the `--no-default-features` build matrix. PRs that fail won't merge.

## Code style

- `cargo fmt` is the formatter.
- `cargo clippy --all-targets -- -D warnings` is the lint gate.
- Doc comments on every public item (`#![warn(missing_docs)]` enforces this).
- Conventional Commits (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`).

## Adding a new feature

1. Open an issue first if it's meaningful. Discuss scope.
2. Branch: `feat/<short-name>` or `fix/<short-name>`.
3. TDD. Real dependencies, no mocks.
4. Update `CHANGELOG.md` under `## [Unreleased]`.
5. PR. Reference the issue.

## Releasing (maintainer-only)

1. Land changes on `main` via PR.
2. Move `## [Unreleased]` items under a new `## [X.Y.Z] - YYYY-MM-DD` heading in `CHANGELOG.md`.
3. Bump `Cargo.toml` version.
4. `cargo publish --dry-run` to confirm clean.
5. `git tag -a vX.Y.Z -m "..."` and `git push origin main vX.Y.Z`.
6. Create GitHub release; paste CHANGELOG entry as notes.
7. `cargo publish`.

## Questions

Open an issue.
