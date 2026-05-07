# ADR-0002: semrouter is a pure classifier; risk policy lives in the consumer

**Status:** Accepted (2026-05-06, codified in v0.1.1)

## Context

Earlier semrouter prototypes tracked a "risk policy" — the router would emit a `policy` block on `RouteDecision` indicating whether a route required user confirmation, was high-risk, etc. The classification was driven by hardcoded substring matching against route names (`"execute_shell_command"` → high-risk, `"send_email"` → requires_confirmation, etc.).

Two problems with this:

1. **The router never actually acted on the policy.** It emitted metadata; the consumer was always going to have to gate dispatch themselves. The policy block was decorative.
2. **The hardcoded substring rules were domain-specific.** Routes named `"unlock_front_door"` aren't in the substring list, so the router would mark them low-risk despite obviously needing confirmation. Each consumer's domain has different routes; the library cannot know what's risky.

## Decision

**semrouter is a pure classifier.** Given an input string and a corpus of labeled examples, it returns:

- `selected_route: Option<String>`
- `status: DecisionStatus` (one of `Accepted`, `Ambiguous`, `BelowThreshold`, `NeedsReview`)
- `confidence: ConfidenceOutput { top_score, second_score, margin }`
- `candidates: Vec<CandidateOutput>`

That's it. No `policy`, no `risk`, no `requires_confirmation` field on `RouteDecision`. The four-status taxonomy gives consumers everything they need to gate their own dispatch.

**Risk classification, confirmation gating, audit logging, and observability live in the consumer's plugin / capability layer.** A typical consumer has a `Plugin` trait like:

```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn examples(&self) -> &'static [&'static str];
    fn risk(&self) -> RiskLevel { RiskLevel::Low }
    fn requires_confirmation(&self) -> bool { false }
    fn handle(&self, input: &str, ctx: &Context) -> Result<Reply>;
}
```

The risk logic sits **next to the dangerous code**, where the author cannot forget to think about it. semrouter's job ends when the route name is returned.

### What semrouter still tracks

- Score thresholds (`minimum_score`, `minimum_margin` from `router.toml`) — these are about **classification confidence**, not policy.
- Hard negatives (counter-examples that penalize specific routes for specific inputs) — also classification, not policy.
- Latency metrics — observability of the classifier itself.

These all sit on the classification side of the line.

## Consequences

### Positive

- semrouter API is small and stable. The four `DecisionStatus` values cover every case; adding policy semantics would mean evolving the enum.
- Risk policies live where they have actual context (the plugin code) instead of in a config file that drifts from reality.
- Future "policy-aware router" work can wrap `SemanticRouter` in a separate crate (potentially with an LLM judge) without bloating semrouter.

### Negative

- Consumers who want a policy layer must build it themselves. Common building blocks (the `Plugin` trait, a `Dispatcher` wrapper) are documented in `docs/integration-example.md` but not provided as code.
- The simple "give me the route" use case is unchanged; the cost falls on consumers who want richer policies, which is fair.

### Neutral

- A future `semrouter-policy` (or similarly-named) wrapper crate could ship the Plugin trait + Dispatcher pattern as reusable code. Out of scope for v0.1.x.

## References

- CHANGELOG.md v0.1.0 (the risk-policy removal, before public release)
- `src/decision.rs` — the `DecisionStatus` enum and `RouteDecision` struct (no `policy` field)
- `docs/integration-example.md` — the Plugin / Dispatcher pattern for consumers
