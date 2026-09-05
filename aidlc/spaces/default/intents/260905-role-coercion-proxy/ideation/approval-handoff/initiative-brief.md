# Initiative Brief — qwaude-proxy (role-coercion-proxy)

## Intent and Problem Statement

vLLM's chat template raises `raise_exception("System message must be at the beginning")` whenever a `role: "system"` message appears anywhere in the `messages` array except index 0 [memory:M1]. Something upstream — the Anthropic-to-OpenAI adapter and/or an agentic tool-context-refresh step — re-injects system-role messages mid-conversation on later turns, which crashes tokenization and breaks the `Claude Code → adapter → Bifrost → vLLM` pipeline [memory:M1]. The initiative exists to let Claude Code run reliably against self-hosted Qwen-family models through this pipeline [memory:M1].

## Market Validation

Not applicable — this is an internal engineering fix for a self-hosted pipeline, not a market-facing product. Market Research was not run for this workflow [memory:M1].

## Feasibility and Risk Highlights

Feasibility & Constraint Analysis was not run as a separate stage for this workflow — the technical approach (an axum/hyper/tokio Rust proxy with a targeted JSON transform) is a well-understood pattern, not a novel technical bet [memory:M3]. One open risk was flagged by the product-lead review at Intent Capture: the benchmark success criterion ("adds negligible latency/allocation overhead versus a naive baseline") had no numeric pass/fail threshold [memory:M1]. Resolution: accepted as a qualitative criterion for now; a specific numeric threshold will be set once NFR Requirements/NFR Design has real measurements to work from [Q2].

## Scope Boundary

In scope: a single Rust binary implementing the full described proxy — HTTP server, request transform (role coercion + both content shapes + unknown-field round-trip), streaming SSE pass-through, verbatim non-streaming response passthrough, zero-copy/low-overhead design, structured logging, environment-variable configuration, graceful shutdown, no-panic error handling, and the full test/benchmark suite [memory:M3]. Out of scope: any deployment, infrastructure, or CI/CD automation work; team formation; any user interface; response-shape translation [memory:M3].

## Concept Visuals

Not applicable — no user interface is in scope; Rough Mockups was not run for this workflow [memory:M3].

## Team Plan

Solo project. The project owner is the sole stakeholder and decision-maker; no formal team, mob composition, or reporting cadence [memory:M2]. Team Formation was not run for this workflow.

## Go/No-Go Recommendation

**Go.** The intent, problem statement, and scope are confirmed by the project owner [Q1]. The one open risk (benchmark threshold) has an accepted resolution path that does not block starting Inception [Q2]. Proceeding straight to Practices Discovery is the confirmed next step [Q3].

## Assumptions & Open Questions

None.
