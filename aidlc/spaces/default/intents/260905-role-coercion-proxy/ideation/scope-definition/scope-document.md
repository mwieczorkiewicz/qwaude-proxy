# Scope Document — qwaude-proxy (role-coercion-proxy)

## Minimum Viable Scope

The full spec described in the initial request is the MVP — there is no smaller "v0" that would fix the crash safely [Q1]. The MVP is a single Rust binary that: listens as an OpenAI-compatible HTTP server, forwards to a configurable vLLM base URL, rewrites mid-conversation `system`-role messages before forwarding, streams SSE responses through without full buffering, passes non-streaming responses through verbatim, is built around a zero-copy/low-overhead JSON handling design, logs coercion activity via `tracing` without leaking payload content by default, is configured entirely via environment variables, shuts down gracefully on SIGINT/SIGTERM, and never panics or unwraps on the request path [desc], [Q1].

## In Scope

- HTTP server exposing `POST /v1/chat/completions`, OpenAI-compatible schema [desc]
- Forwarding to a configurable `VLLM_BASE_URL` [desc]
- Request transform: leave `messages[0]` untouched if `role: "system"`; for every other `system`-role message, rewrite `role` to `"user"` and prepend a configurable notice prefix to its text content; handle both plain-string and content-block-array content shapes; round-trip every other field (tool_calls, tool_call_id, name, cache-control hints, and any unknown fields) unchanged [desc]
- Streaming (`"stream": true`) pass-through of the upstream SSE byte stream without buffering the full response, preserving backpressure [desc]
- Verbatim pass-through of non-streaming responses (status code, relevant headers, body) [desc]
- Zero-copy / low-overhead JSON handling as the primary design constraint, with any unavoidable full-JSON-round-trip compromise documented and benchmarked [desc]
- Structured `tracing` logging: per-request coerced-message count and original indices at `debug`; upstream errors at `error`; no payload content logged by default, with an opt-in verbose-payload-logging flag for local debugging [desc]
- Environment-variable configuration (`LISTEN_ADDR`, `VLLM_BASE_URL`, `NOTICE_PREFIX`, `LOG_LEVEL`) with documented defaults [desc]
- Graceful shutdown on SIGINT/SIGTERM, draining in-flight requests including long-lived streaming ones [desc]
- No `unwrap()`/`panic!()` on the request path; malformed input, unexpected schema shapes, and upstream failures return appropriate HTTP status codes with a small JSON error body [desc]
- Unit tests, property-based tests (proptest/quickcheck), integration tests against a mock upstream, and a criterion benchmark comparing the zero-copy transform against a naive full-deserialize baseline [desc]
- `cargo clippy -D warnings` and `cargo fmt --check` clean [desc]
- `Cargo.toml`, `src/` (with `transform.rs` isolated from `main.rs`), `tests/`, `benches/`, and a `README.md` covering the problem, pipeline position, configuration, and how to run tests/benchmarks [desc]

## Out of Scope

- Any deployment, infrastructure, or CI/CD automation work [memory:M1] (confirmed in Intent Capture, Q6)
- Team formation or a multi-person delivery team — this is a solo build [memory:M1] (confirmed in Intent Capture, Q5)
- Any user interface [desc]
- Response-shape translation (this proxy only rewrites request message roles, not the response format) [desc]
- No additional out-of-scope items beyond the above were identified [Q5]

## Must-Have vs. Nice-to-Have (MoSCoW)

Everything listed under **In Scope** is a **Must Have** — this is a correctness-and-reliability fix where the transform logic, streaming pass-through, no-panic error handling, and the zero-copy design constraint are all non-negotiable together [Q2]. No **Should Have**, **Could Have**, or **Won't Have** items were identified.

## Value Stream Map

| Capability | Customer Outcome | Source |
|---|---|---|
| Request transform (role coercion) | The `Claude Code → adapter → Bifrost → vLLM` pipeline no longer crashes on mid-conversation system messages, so agentic sessions complete reliably | [desc], [memory:M1] |
| Streaming pass-through | No added latency or buffering overhead is introduced for streamed responses, preserving the interactive agentic experience | [desc] |
| Zero-copy design + benchmark | Confidence that the fix does not introduce a meaningful performance regression on this internal request path | [desc], [memory:M1] |
| Test suite + CI-clean lint/format | Confidence the fix is correct across content shapes, edge cases, and won't regress silently | [desc] |

## Sequencing / Dependencies

Risk-first, dependency-ordered: the message-array transform logic and its zero-copy design are built and tested first (isolated in `transform.rs`, unit- and property-testable without a server), since that is where the real design risk lives given serde's ownership model. Then the HTTP server is wired (forwarding, streaming pass-through, response passthrough), then configuration, logging, graceful shutdown, and error handling, then the full integration test suite and the criterion benchmark validate all of it together [Q3].

## Assumptions & Open Questions

None.
