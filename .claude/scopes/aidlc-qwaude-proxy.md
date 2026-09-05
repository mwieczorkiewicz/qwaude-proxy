---
name: qwaude-proxy
depth: Standard
keywords: []
description: Rust role-coercion proxy between Bifrost and vLLM
skeleton: off
review_cap: advisory
---

# qwaude-proxy scope

Standard depth composed for a single, already-fully-specified Rust backend
service: a role-coercion proxy that sits between Bifrost and vLLM. The
requirements arrived pre-written in exhaustive detail, so the plan strips
ideation/requirements-gathering ceremony down to a thin trigger record and
puts the weight on design-of-the-transform, code generation, and thorough
testing/benchmarking instead.

## Why these stages, why skip those

Composed by the adaptive-workflows composer for a task whose own prompt
already **is** the requirements spec (exact transform rules, endpoint,
config, error codes, test types, deliverable layout). Intent-capture and
scope-definition run cheaply to record the trigger and the explicit
boundary ("no UI, no new team formation"); requirements-analysis,
user-stories, market-research, rough/refined-mockups, and team-formation are
all skipped because the task text already answers what they would produce
and there is no market, no UI, and no team to form. Reverse-engineering is
skipped because the project is greenfield (no existing code to map).
Feasibility folds into domain-design — an axum/hyper/tokio proxy with a
zero-copy JSON transform and SSE passthrough is a well-documented Rust
pattern, not a novel bet needing its own viability pass. Practices-discovery
still runs because this is a brand-new crate with tooling choices (Rust
edition, axum/hyper/tokio and test-crate versions, clippy/fmt config) that
need affirming into team.md even though the task pre-specifies most of them.
Domain-design, nfr-requirements, and nfr-design carry the real design weight
— module boundaries (main.rs/transform.rs isolation), and the interacting
non-functional constraints (zero-copy allocation, SSE streaming, structured
no-payload logging, graceful shutdown, error taxonomy) that the task calls
out as its primary design constraint. Units-generation, contract-design, and
delivery-planning are skipped because this is one unit with a fixed external
wire contract (OpenAI-compatible chat completions) and nothing to sequence.
Functional-design is skipped because the transform logic is one bounded,
already-fully-specified module. Infrastructure-design is skipped because no
infrastructure or deployment work was requested and none of the operation-
phase deployment/environment stages survive the screen. Code-generation and
build-and-test are the spine; ci-pipeline runs to wire clippy -D warnings,
fmt --check, and the full test suite into automated per-change enforcement.
Every operation-phase stage (deployment, environment provisioning,
observability, incident-response, performance-validation,
feedback-optimization) is skipped: the task asks for a buildable, tested,
benchmarked crate with a README, not a deployed/operated service.

## Membership

`keywords: []` — this is a composed scope with no human-granted inference
triggers. It resolves only by explicit `--scope qwaude-proxy` (or by being
the scope an intent was created against). Initialization, intent-capture,
scope-definition, approval-handoff, practices-discovery, domain-design,
nfr-requirements, nfr-design, code-generation, build-and-test, and
ci-pipeline execute (13 of 33 stages); everything else is SKIP.
