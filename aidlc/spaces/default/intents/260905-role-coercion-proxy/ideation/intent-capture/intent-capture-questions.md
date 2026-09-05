# Intent Capture — Questions

## Sources

- [desc] Initial description: "Build a production-quality Rust proxy (role-coercion-proxy) that sits between Bifrost and vLLM, rewriting mid-conversation system-role messages to fix a chat-template validation crash. Full detailed spec already provided by the user: HTTP server (axum/hyper/tokio), POST /v1/chat/completions, forwards to configurable VLLM_BASE_URL, transforms request JSON (leaves messages[0] system alone, coerces other system messages to user role with a prefix, handles both string and content-block-array content shapes, round-trips unknown fields), streaming SSE pass-through without full buffering, verbatim non-streaming response passthrough, zero-copy/low-allocation JSON handling as the primary design constraint (documented compromises where serde's ownership model forces an allocation), structured tracing logging (debug-level coercion counts/indices, no payload content by default), env var configuration with documented defaults, graceful shutdown on SIGINT/SIGTERM, no panics/unwraps on the request path with proper 400/502/504 error handling. Testing: unit tests, proptest property tests for invariants, integration tests (wiremock or axum test server) covering streaming/non-streaming/malformed input/upstream failure/large payloads, criterion benchmark comparing zero-copy vs naive full-deserialize baseline, clippy -D warnings and fmt --check clean. Deliverables: Cargo.toml, src/ (transform.rs isolated from main.rs), tests/, benches/, README.md documenting the problem, pipeline position, config, and how to run tests/benchmarks. This is a single well-scoped backend service with no UI, no new team formation, and requirements already fully specified — minimize ideation/requirements-gathering ceremony and focus process on design-of-the-transform, code generation, and thorough testing/benchmarking."
- [scope] Workflow-selected scope: `qwaude-proxy`.

## Q1. What business problem are we solving?

The description already gives a precise technical account. Confirming it as the framing for downstream artifacts.

A. As described: vLLM's chat template raises `raise_exception("System message must be at the beginning")` whenever a `role: "system"` message appears anywhere in the `messages` array except index 0. Something upstream (the Anthropic-to-OpenAI adapter and/or an agentic tool-context-refresh step) re-injects system-role messages mid-conversation on later turns, which crashes tokenization and breaks the `Claude Code → adapter → Bifrost → vLLM` pipeline.
B. Narrower than that — only a specific subset of the described crash matters
C. Broader than that — this proxy should also solve other request-shaping problems beyond the system-role crash
D. Not yet defined
X. Other (please specify)

[Answer]: A. Basically we want to use Claude Code with Self-Hosted Qwen-Family Models.

## Q2. Who is the customer (internal/external)? What pain are they experiencing?

A. Internal only: the engineer(s) operating this self-hosted `Claude Code → Bifrost → vLLM` pipeline. Pain = agentic sessions crash/fail whenever a mid-conversation system message reaches vLLM.
B. Internal + external: this pipeline also serves other users/teams beyond the operator
C. Not yet defined
D. Not applicable
X. Other (please specify)

[Answer]: Internal only. Other users - engineering team, but for the sake of implementation - assume internal.

## Q3. What does success look like? What metrics matter?

A. Zero tokenizer crashes in production traffic from mid-conversation system-role messages; verified by the full test suite (unit + property-based + integration) passing and `cargo clippy -D warnings` / `cargo fmt --check` staying clean; the criterion benchmark confirms the zero-copy transform adds negligible latency/allocation overhead versus a naive full-rebuild baseline
B. Something else
C. Not yet defined
X. Other (please specify)

[Answer]: A. 

## Q4. What is the trigger for this initiative (market pressure, tech debt, regulation, opportunity)?

A. Active production incident: mid-conversation system-role messages are crashing the pipeline today
B. Proactive risk mitigation: not yet observed in production, but a known failure mode being closed off before it bites
C. Not yet defined
X. Other (please specify)

[Answer]: Research and development activities.

## Q5. Who are the key stakeholders, who decides scope/priority, and are there any communication requirements or reporting cadence?

A. Solo project: you are the sole stakeholder and decision-maker; no formal team, no reporting cadence, no status updates needed beyond this workflow's own approval gates
B. Small team — I'll specify who's involved and what they care about
C. Not yet defined
D. Not applicable
X. Other (please specify)

[Answer]: A

## Q6. The workflow was started with scope `qwaude-proxy` (13 of 33 stages: intent capture, scope definition, approval handoff, practices discovery, domain design, NFR requirements, NFR design, code generation, build-and-test, CI pipeline — no market research, no team formation, no UI, no deployment/infra automation). Does that match your intended product boundary?

A. Yes, confirmed — a single Rust proxy binary (code + tests + benchmarks + README) is the entire deliverable; no deployment, infra, or team-formation work is in scope
B. No — I want to expand the boundary to include deployment/infra work
C. No — I want to narrow the boundary further
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]:
