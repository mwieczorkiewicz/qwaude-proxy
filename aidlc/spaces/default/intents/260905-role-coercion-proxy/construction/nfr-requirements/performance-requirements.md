# Performance Requirements — qwaude-proxy (role-coercion-proxy)

## NFR1: Performance

| ID | Requirement | Target | Measurement |
|---|---|---|---|
| NFR1.1 | Latency overhead attributable to this proxy's own processing (transform + routing), relative to a direct pass-through baseline with no transform applied | < 5% p99 latency overhead | Criterion benchmark comparing the zero-copy transform path against a naive full-deserialize/full-rebuild baseline; tracked and reported in Build-and-Test, not a hard CI gate (per affirmed team practices) |
| NFR1.2 | Additional heap allocations per request on the fast path (no message actually needs coercion — e.g. a single leading system message or none at all) | Near-zero additional allocations beyond what parsing/reserialization already requires | Criterion benchmark allocation counting (e.g. via a custom allocator or `dhat`), compared against the naive baseline |
| NFR1.3 | Additional heap allocations when coercion actually occurs | A modest, bounded, and documented increase — rewriting `role` and prepending the notice prefix necessarily allocates the new string content | Same benchmark; document the exact allocation cost in code comments per the coding task's own requirement to explain compromises |
| NFR1.4 | Concurrent request handling | At least tens of concurrent requests without one slow/streaming client degrading another's latency | Async, non-blocking I/O throughout (tokio); no target requests/second — modest internal load only (Q2) |
| NFR1.5 | Streaming first-byte latency | SSE pass-through must not introduce artificial buffering delay — the first byte of the upstream response reaches the client without waiting for the full body | Integration test asserting the client starts receiving bytes before the mock upstream finishes sending (already specified in the original task's test requirements) |

## Source

Q1, Q2 (`nfr-requirements-questions.md`). NFR1.1 resolves the open item flagged by the product-lead review at Intent Capture (finding R-01): the original success criterion had no numeric threshold.

## Assumptions & Open Questions

None.
