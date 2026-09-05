# Reliability Design — qwaude-proxy (role-coercion-proxy)

## NFR4.1 — Fail-fast upstream failure handling, with configurable timeouts

**Design**: the outbound `hyper` client to vLLM is configured with two timeouts, both overridable via environment variables (confirmed Q2), defaulting to:
- **Connect timeout**: 5s — bounds how long the proxy waits to establish the TCP connection to `VLLM_BASE_URL`.
- **Total request timeout**: 30s for non-streaming responses.

For a streaming response (`"stream": true`), the timeout window covers only connection establishment and receiving the first byte of the response — once streaming begins, no additional deadline applies, so a legitimately long completion is never killed mid-stream (confirmed Q2). A timeout or connection failure at any point before the first byte returns `502`/`504` immediately via `ProxyError`, with no retry (NFR4.1) — see `security-design.md` NFR2.6 for the response body shape.

```rust
// pseudocode - interface-level, not implementation-ready
let client = Client::builder()
    .connect_timeout(config.connect_timeout) // default 5s, env-configurable
    .build();
let response = tokio::time::timeout(config.total_timeout, client.request(req)).await
    .map_err(|_| ProxyError::UpstreamTimeout)??;
// once `response` headers/first byte are received, the stream body is forwarded
// with no further per-chunk deadline.
```

## NFR4.2 — Graceful shutdown

**Design**: axum's `with_graceful_shutdown` is fed a future that resolves on either SIGINT or SIGTERM (`tokio::signal::ctrl_c()` and `tokio::signal::unix::signal(SignalKind::terminate())`, raced with `tokio::select!`). On signal, the server stops accepting new connections and waits for in-flight requests — including long-lived streaming ones — to complete naturally before the process exits; no in-progress response is dropped mid-stream.

## NFR4.3 — No crash on malformed or unexpected input

**Design**: every fallible operation on the request path returns `Result<T, TransformError>` or `Result<T, ProxyError>` and is propagated with `?` up to the single top-level request handler, which is the only place an error is converted into an HTTP response (via the `IntoResponse` implementation in `security-design.md` NFR2.6). No `.unwrap()`/`.expect()`/`panic!()` exists on this path — mechanically enforced by the clippy deny-lints (`team-practices.md`), not just this design convention.

## NFR4.4 — No availability SLA/SLO

Not applicable — restated from `reliability-requirements.md`; no design work needed since there is no deployed, monitored instance in this workflow's scope.

## Support Perspective (AWS Platform)

No multi-AZ, health-check-driven auto-restart, or failover design applies — there is no deployment topology in this workflow's scope to design resilience infrastructure against. The application-level reliability patterns above (timeouts, graceful shutdown, no-panic) are the entirety of this workflow's reliability design.

## Source

Q2 (`nfr-design-questions.md`); `reliability-requirements.md` (NFR4.1-4.4); `team-practices.md` (clippy deny-lints).

## Assumptions & Open Questions

None.
