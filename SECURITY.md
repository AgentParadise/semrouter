# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in semrouter, please report it privately. **Do not open a public issue.**

**Preferred channel:** [GitHub Security Advisories](https://github.com/AgentParadise/semrouter/security/advisories/new) — this creates a private discussion with the maintainers.

**Alternative:** Open a private issue or DM a maintainer on GitHub.

We will:
- Acknowledge receipt within 7 days.
- Investigate and confirm or deny the issue within 30 days.
- Coordinate disclosure timing once a fix is ready.
- Credit the reporter in the security advisory unless they prefer to remain anonymous.

## Scope

Vulnerabilities we consider in scope:

- Memory safety issues in unsafe code (we have ~zero unsafe; any is in scope)
- Arbitrary code execution via crafted `routes.jsonl` / `eval.jsonl` / `router.toml` inputs
- Denial-of-service via inputs that cause unbounded resource consumption
- Supply-chain issues (e.g. a transitive dependency CVE we should pin away from)

Out of scope:
- The accuracy or quality of routing decisions (that's an evaluation question, not security).
- Issues in third-party embedders the consumer brings via the `EmbeddingProvider` trait.
- Reports that boil down to "your dep tree includes crate X which has known issue Y" — please file these against the upstream crate; we follow security advisories via cargo-audit and Dependabot.

## Versions

We provide security updates for:

- The latest published `0.x` version on crates.io.
- The `main` branch of the GitHub repo.

Older versions are not supported. If you need a fix for a specific version, open an issue.
