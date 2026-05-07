# Architecture Decision Records (ADRs)

This directory holds non-obvious design decisions that future maintainers (and current readers) should understand.

Format: [Michael Nygard's ADR template](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions). Each ADR is short (Status, Context, Decision, Consequences) and dated.

| # | Title | Status |
|---|---|---|
| 0001 | [Zero default dependencies as a non-negotiable goal](0001-zero-dependencies-goal.md) | Accepted |
| 0002 | [semrouter is a pure classifier; risk policy lives in the consumer](0002-pure-classifier-architecture.md) | Accepted |
| 0003 | [BYO embedder via the EmbeddingProvider trait](0003-byo-embedder-trait.md) | Accepted |

## When to write a new ADR

- Adding or removing a default dependency (always)
- Architectural changes that span multiple modules (probably)
- Public API decisions that constrain future evolution (probably)
- Style or formatting preferences (no, those go in `CONTRIBUTING.md`)

## How to write one

1. Copy an existing ADR as a template.
2. Number it sequentially (next free integer; never reuse).
3. Set Status to `Proposed` initially; flip to `Accepted` when merged.
4. Keep it short. Two pages of markdown is a long ADR.
5. Reference it from CHANGELOG, README, or commits where the decision is invoked.
