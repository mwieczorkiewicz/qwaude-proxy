# Tech Stack Decisions — qwaude-proxy (role-coercion-proxy)

## HTTP Server Framework: axum (+ hyper + tokio)

**Decision**: `axum`, built on `hyper` and `tokio`.

**Rationale**: Matches the preference stated in the original task spec. Axum's `tower`-based middleware ecosystem, native streaming-body support (`axum::body::Body` composes directly with `hyper`'s streaming primitives), and first-class SSE handling patterns fit this proxy's core requirement (streaming pass-through without full buffering) directly. It also shares its HTTP stack with `hyper`, which this proxy uses again for the outbound connection to vLLM — one stack, not two.

**Alternatives considered**:
- `actix-web` — comparable performance, but its actor-model runtime is a heavier dependency for a proxy this size, and its request-handling model composes less naturally with a manual forwarding handler than `tower`'s `Service` trait does.
- `warp` — built on similar primitives to axum (also `hyper`/`tokio`-based), but its filter-based routing API is less ergonomic for this proxy's single-route, forwarding-focused use case, and it has smaller ecosystem momentum than axum.

## JSON Handling: simd-json with serde_json fallback

**Decision**: Prefer `simd-json` when SIMD instructions are available on the build/runtime architecture; fall back to `serde_json` otherwise (confirmed Q6). Both back onto a shared targeted-mutation strategy — the transform parses the request, mutates only the affected `messages` entries' `role`/`content` fields, and reserializes; neither is a byte-for-byte in-place patch, since JSON structural changes (e.g. `role: "system"` → `role: "user"`, a differing-length string) cannot be done in place at the byte level.

**Rationale**: `simd-json` requires SIMD support (SSE4.2/AVX2 on x86_64, NEON on aarch64) and mutates its input buffer in place during parsing, which can yield real throughput gains over `serde_json` for larger payloads — directly relevant to this proxy's low-overhead design goal. `serde_json` is the safe, universally-available fallback so the proxy still builds and runs correctly on architectures/targets without the required SIMD instructions, at a modest performance cost on that path.

**Where the ownership model still forces a copy**: neither library achieves a fully zero-copy JSON tree in safe Rust. `serde_json::Value::String` and `simd_json::owned::Value`'s string variant are always heap-allocated `String`s — parsing into either `Value` type allocates a new `String` for every JSON string in the document, borrowed-`&str` slicing of the original buffer notwithstanding for the *unparsed* bytes. The zero-copy property this proxy actually delivers is narrower and more targeted: **untouched messages and untouched fields are never re-allocated or re-copied beyond what parsing itself requires** — the transform mutates only the specific `role`/`content` fields on the specific messages that need coercion, rather than deep-cloning the whole request body and rebuilding it field-by-field. Code Generation documents the exact allocation cost of this compromise (per NFR1.3) with the benchmark's own numbers, per the original task's explicit request to explain any place a true zero-copy implementation wasn't possible.

**Alternative considered**: a manual `nom`-based (or hand-rolled) partial byte-slice parser targeting only the `messages` array, leaving the rest of the request body as an untouched byte range. This has the highest theoretical zero-copy ceiling but was rejected for this workflow given its implementation complexity and risk relative to the confirmed benefit — the `simd-json`/`serde_json` targeted-mutation approach already satisfies NFR1.1/NFR1.2's numeric performance targets without that added risk, per the human's confirmed answer (Q6).

## Async Runtime: tokio

**Decision**: `tokio` (multi-threaded runtime).

**Rationale**: Required by `axum`/`hyper`; no alternative was considered, since the framework choice above fixes this.

## Outbound HTTP Client: hyper (via hyper-util), not reqwest

**Decision**: Use `hyper`'s client directly (via the `hyper-util` client helpers) for the outbound connection to vLLM, rather than `reqwest`.

**Rationale**: This proxy needs precise control over streaming-body pass-through on the outbound leg (SSE chunks flowing straight through to the inbound response without buffering). `hyper`'s client gives direct access to that streaming body type — the same one the inbound `axum` handler already works with — avoiding an impedance mismatch with `reqwest`'s higher-level response-body abstraction. It also keeps the dependency surface smaller: `reqwest` pulls in its own connection-pooling and middleware layers that duplicate what `hyper`/`axum` already provide.

**Alternative considered**: `reqwest` — more ergonomic for typical request/response use cases, but its abstractions work against this proxy's specific streaming-pass-through requirement.

## Metrics: `metrics` crate + `metrics-exporter-prometheus`

**Decision**: The `metrics` facade crate with the `metrics-exporter-prometheus` backend, exposed via an axum route handler for `GET /metrics` (NFR5.3).

**Rationale**: Idiomatic, widely used in the Rust ecosystem, and integrates as a simple axum route rather than requiring a separate server/process.

## Error Handling: thiserror

**Decision**: `thiserror` for the typed per-layer error enums (`TransformError`, `ProxyError`) already affirmed in `team-practices.md` (Code Style, Q6).

**Rationale**: Already decided at Practices Discovery; restated here for completeness of the tech-stack record.

## Testing: proptest, criterion, wiremock

**Decision**:
- `proptest` for property-based testing of `transform.rs` (already affirmed in team practices).
- `criterion` for the benchmark comparing the zero-copy transform against the naive baseline (already specified in the original task).
- `wiremock` for the integration tests' mock upstream, in preference to standing up a local `axum` test server to play the vLLM role.

**Rationale for wiremock over a local axum test server**: `wiremock` doesn't require running an actual second HTTP framework instance to simulate vLLM — it stubs HTTP responses (including chunked/streaming ones) directly, which keeps the integration test setup simpler and faster, and avoids coupling the test harness to axum internals. The original task listed both as acceptable options; this documents the chosen one with rationale, per the Inception phase guardrail requiring trade-off analysis for design decisions.

## Assumptions & Open Questions

None.
