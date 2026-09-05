# NFR Requirements — Questions

This is a single-unit project (no Units Generation ran), so these questions cover the whole proxy rather than one unit. They draw on `components.md`/`decisions.md` (Domain Design) and `team-practices.md` (Practices Discovery).

## Q1. What latency/allocation overhead threshold should the proxy's own processing add, relative to a direct pass-through with no transform?

This resolves an open item flagged by the product-lead review at Intent Capture: the original success criterion ("negligible latency/allocation overhead") had no numeric pass/fail threshold.

A. <5% p99 latency overhead and near-zero additional heap allocations per request on the fast path (no system-role coercion needed); a modest, bounded, and documented allocation increase only when coercion actually happens
B. A different concrete threshold — I'll specify
C. No numeric target — keep "negligible" qualitative; the criterion benchmark's own zero-copy-vs-naive comparison is the standard, with no pass/fail gate
D. Not yet defined
X. Other (please specify)

[Answer]: A

## Q2. What concurrent-request volume should this proxy be designed to handle without degradation?

A. Modest internal load — tens of concurrent requests (a handful of engineers running Claude Code sessions); no specific requests/second target, just don't let one slow streaming client block others (tokio's async model handles this by default)
B. A specific target — I'll specify (e.g. N req/s or N concurrent connections)
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. If the connection to vLLM fails or times out, should the proxy retry automatically, or fail fast?

A. Fail fast — return 502/504 immediately, no retry inside the proxy (keeps the proxy simple, avoids duplicating retry logic Bifrost or Claude Code may already have)
B. Retry with backoff — the proxy itself retries the upstream connection before giving up
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q4. Should the proxy expose a basic health-check endpoint?

Useful for local testing and any future deployment, even though deployment itself is out of scope for this workflow.

A. Yes — a simple `GET /health` returning 200 OK when the process is up (no upstream dependency check)
B. No — not needed for this workflow's scope
C. Not yet defined
X. Other (please specify)

[Answer]: X. Other — a `GET /health` endpoint AND a standard Prometheus metrics endpoint (e.g. `GET /metrics`) exposing a reasonable set of metrics (request counts, latency histogram, coercion counts, upstream error counts)

## Q5. Beyond the already-decided "no payload content logged by default," is there any additional data-protection requirement for request/response bodies?

Claude Code conversations passing through this proxy may contain sensitive content (code, credentials pasted by mistake, etc.).

A. No additional requirement — the proxy already never logs payload content by default and processes everything in-memory per-request with no persistence to disk; that's sufficient given the internal network context
B. Yes — additional requirement, I'll specify
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q6. Confirm the technical approach for the zero-copy JSON transform (the core design constraint).

A. `serde_json::Value`-based: parse into a `Value` tree (borrowing string data where serde's API allows), mutate only the affected `messages` array entries' `role`/`content` fields in place, and reserialize — with the specific points where serde's ownership model forces a copy (e.g. `Value::String` is always owned; a fully zero-copy JSON tree isn't achievable with serde alone) documented in code comments and the benchmark
B. `simd-json`-based: same targeted-mutation strategy, using simd-json's SIMD-accelerated parser for faster throughput on the parse step specifically
C. A manual byte-slice partial parser for just the `messages` array — highest zero-copy potential, highest implementation risk/complexity
D. Not yet defined
X. Other (please specify)

[Answer]: X. Other — prefer `simd-json` when SIMD support is available on the build/runtime architecture, falling back to `serde_json` when it isn't; same targeted-mutation strategy either way
