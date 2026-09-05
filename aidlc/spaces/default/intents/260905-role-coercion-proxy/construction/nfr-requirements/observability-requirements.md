# Observability Requirements — qwaude-proxy (role-coercion-proxy)

## NFR5: Observability

| ID | Requirement | Detail |
|---|---|---|
| NFR5.1 | Structured logging | Via `tracing`: per-request coerced-message count and original indices at `debug`; upstream errors at `error`; no payload content logged by default, with an opt-in verbose-payload-logging flag for local debugging only (already specified in the original task; restated here as a formal NFR) |
| NFR5.2 | Health endpoint | `GET /health` returns `200 OK` when the process is up — no upstream dependency check (i.e. it reflects "the proxy process is alive," not "vLLM is reachable") (confirmed Q4) |
| NFR5.3 | Metrics endpoint | `GET /metrics` exposes Prometheus-format metrics (confirmed Q4): request count (labeled by status code), request latency histogram, coercion count (messages coerced per request and cumulative total), upstream error count (labeled by failure kind: connection refused, timeout, non-2xx status) |
| NFR5.4 | No distributed tracing | Not required — this is a single-hop proxy (one inbound request, one outbound request per invocation); there is no multi-service trace to propagate context across, so W3C Trace Context propagation and span-based distributed tracing are out of scope for this workflow |

## Logging Levels

| Level | When to Use | Notes |
|---|---|---|
| ERROR | Upstream connection failures, unexpected internal errors | Never includes payload content |
| DEBUG | Per-request coercion counts/indices | Never includes payload content unless verbose logging is explicitly enabled |
| INFO/WARN | Not specifically required beyond the above; may be used for startup/shutdown lifecycle events (e.g. "listening on `<addr>`", "shutdown signal received, draining N in-flight requests") | Ordinary operational logging, no payload content |

## Source

Q4 (`nfr-requirements-questions.md`); original task spec (NFR5.1); Ideation scope-document.md (no deployment/observability-platform work in scope, hence NFR5.4).

## Assumptions & Open Questions

None.
