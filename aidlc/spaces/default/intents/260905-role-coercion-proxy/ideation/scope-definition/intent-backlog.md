# Intent Backlog — qwaude-proxy (role-coercion-proxy)

Prioritized using MoSCoW [Q2]. Every item below is a **Must Have** — no Should/Could/Won't items were identified for this scope. Sequenced risk-first per [Q3]. Since this is a single-unit deliverable [scope], these are proto-capability groups within that one unit, not separate Units of Work — Units Generation (2.7) will confirm the final decomposition.

## Backlog

| # | Capability | Priority | Sequence Rationale | Source |
|---|---|---|---|---|
| 1 | Message-array transform logic (role coercion, both content shapes, unknown-field round-trip), isolated in `transform.rs` | Must | First — highest design risk (serde ownership model); unit/property-testable in isolation | [desc], [Q1], [Q3] |
| 2 | HTTP server + request forwarding (`POST /v1/chat/completions`, configurable `VLLM_BASE_URL`) | Must | Second — depends on #1 being callable from the request path | [desc], [Q3] |
| 3 | Streaming SSE pass-through (no full buffering, backpressure preserved) | Must | Third — builds on the HTTP wiring in #2 | [desc], [Q3] |
| 4 | Non-streaming response pass-through (verbatim) | Must | Alongside #3 — same HTTP-wiring dependency | [desc] |
| 5 | Configuration (env vars: `LISTEN_ADDR`, `VLLM_BASE_URL`, `NOTICE_PREFIX`, `LOG_LEVEL`) | Must | Fourth — needed before logging/shutdown can be exercised meaningfully | [desc], [Q3] |
| 6 | Structured `tracing` logging (debug-level coercion counts/indices, error-level upstream failures, no payload content by default) | Must | Fourth (parallel with #5) | [desc], [Q3] |
| 7 | Graceful shutdown (SIGINT/SIGTERM, drain in-flight and streaming requests) | Must | Fourth (parallel with #5-#6) | [desc], [Q3] |
| 8 | Error handling (no panics/unwraps on the request path; 400/502/504 with JSON error body) | Must | Fourth (parallel with #5-#7) | [desc], [Q3] |
| 9 | Unit tests + property-based tests (proptest/quickcheck) for the transform | Must | Fifth — validates #1 | [desc], [Q3] |
| 10 | Integration tests against a mock upstream (streaming, non-streaming, malformed input, upstream failure, large payloads) | Must | Fifth — validates #2-#8 together | [desc], [Q3] |
| 11 | Criterion benchmark (zero-copy transform vs. naive full-deserialize baseline) | Must | Fifth — validates the zero-copy design constraint end to end | [desc], [Q1], [Q3] |
| 12 | `cargo clippy -D warnings` / `cargo fmt --check` clean | Must | Continuous, verified at the end | [desc] |
| 13 | `README.md` (problem, pipeline position, config, how to run tests/benchmarks) | Must | Last — documents the finished system | [desc] |

## Assumptions & Open Questions

None.
