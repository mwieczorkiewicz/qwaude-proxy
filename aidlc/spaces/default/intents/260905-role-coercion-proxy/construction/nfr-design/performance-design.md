# Performance Design — qwaude-proxy (role-coercion-proxy)

## NFR1.1 / NFR1.2 / NFR1.3 — Latency and allocation overhead

**Design**: `Transform` dispatches at runtime between `simd-json` and `serde_json` based on detected CPU SIMD support (confirmed Q1 = Option B), so a single binary runs optimally across deployment targets without a recompile:

```rust
// pseudocode - runtime dispatch, not implementation-ready
fn parse_and_coerce(bytes: &[u8], prefix: &str) -> Result<Vec<u8>, TransformError> {
    if simd_json_supported() {
        transform_simd(bytes, prefix)
    } else {
        transform_serde(bytes, prefix)
    }
}
```

Both paths follow the same targeted-mutation strategy: parse once, locate the `messages` array, mutate only the entries that need coercion (`role` + prepended-prefix `content`), leave every other message and field untouched, reserialize. Neither path deep-clones the whole request body field-by-field — the "zero-copy" property is this targeted-mutation discipline, not a byte-for-byte in-place patch (see `tech-stack-decisions.md` for where the ownership model still forces a copy).

**Connection reuse**: the outbound HTTP client to vLLM uses `hyper`'s built-in keep-alive connection pooling — one persistent connection is reused across requests to the same `VLLM_BASE_URL` host rather than reconnecting per request, avoiding TCP/TLS handshake overhead on the hot path.

## NFR1.4 — Concurrency

**Design**: `tokio`'s multi-threaded runtime (the default `#[tokio::main]` executor) schedules concurrent request-handling tasks across available CPU cores; no explicit connection or concurrency limit is configured at this proxy's confirmed scale (modest internal load — tens of concurrent requests). Each request is handled by an independently scheduled async task, so one slow streaming client's I/O wait never blocks another request's progress.

## NFR1.5 — Streaming first-byte latency

**Design**: the outbound response body from vLLM (a `hyper` streaming body) is forwarded directly into the axum response as a streamed body (`axum::body::Body::from_stream` over the upstream response's byte-stream), chunk by chunk, as bytes arrive — never collected into an intermediate buffer. This is the same mechanism whether the upstream response is SSE (`stream: true`) or an ordinary chunked response; only the passthrough for non-streaming JSON responses reads the full (already-small, non-streaming) body before forwarding it verbatim, per the original task's requirement that non-streaming responses pass through as-is.

## Support Perspective (AWS Platform)

No caching tier, CDN, or infrastructure-level performance optimization applies — this proxy has no deployment/infrastructure work in scope for this workflow, and its performance characteristics are entirely a function of the binary's own code (async I/O, connection reuse, streaming pass-through), not a platform decision.

## Source

Q1, Q2 (`nfr-design-questions.md`); `performance-requirements.md` (NFR1.1-1.5).

## Assumptions & Open Questions

None.
