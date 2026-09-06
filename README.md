# role-coercion-proxy

An HTTP proxy that fixes a specific incompatibility between an
Anthropic-shaped tool-calling pipeline and vLLM's chat template.

## The problem

```
Claude Code -> Anthropic-to-OpenAI adapter -> Bifrost (auth/routing) -> [this proxy] -> vLLM (Qwen3.6)
```

vLLM's chat template raises `raise_exception("System message must be at the
beginning")` whenever a `role: "system"` message appears anywhere in the
`messages` array except index 0. Upstream adapters in this pipeline
sometimes inject additional system-role messages mid-conversation (tool
reminders, policy nudges), which vLLM's tokenizer then rejects outright.

`role-coercion-proxy` sits directly in front of vLLM and rewrites the
request just enough to avoid that: it leaves `messages[0]` alone, and for
every *other* message whose `role` is `"system"`, it rewrites `role` to
`"user"` and prepends a configurable notice prefix to that message's
content. Nothing else about the request is touched, and non-streaming
responses, streaming responses, and error conditions from vLLM are all
passed straight through.

## Running it

```bash
cargo run --release
```

The proxy listens on `LISTEN_ADDR` (default `0.0.0.0:8080`) and forwards to
`VLLM_BASE_URL` (default `http://127.0.0.1:8000`). Point Bifrost's outbound
model URL at this proxy instead of at vLLM directly.

## Configuration

All configuration is via environment variables; every one has a documented
default and the process runs with no environment variables set at all.

| Variable | Default | Meaning |
|---|---|---|
| `LISTEN_ADDR` | `0.0.0.0:8080` | Address:port the HTTP server binds to. |
| `VLLM_BASE_URL` | `http://127.0.0.1:8000` | Base URL of the upstream vLLM server; `/v1/chat/completions` is appended. |
| `NOTICE_PREFIX` | `[System Notification] ` | Prepended to a coerced system message's content. |
| `LOG_LEVEL` | `info` | `tracing_subscriber::EnvFilter` directive (`trace`/`debug`/`info`/`warn`/`error`, or a per-module directive string). Falls back to `info` if the value doesn't parse as a filter. |
| `MAX_REQUEST_BODY_SIZE` | `10485760` (10 MiB) | Maximum request body size in bytes; larger bodies get a `413`. |
| `UPSTREAM_CONNECT_TIMEOUT_SECS` | `5` | Seconds to wait for the TCP connect to `VLLM_BASE_URL` before failing with a `502`. |
| `UPSTREAM_TOTAL_TIMEOUT_SECS` | `30` | Seconds to wait for the upstream response *headers* (not the full body) before failing with a `504`. A streaming response is therefore only bounded until its first byte, never for the duration of the stream. |
| `VERBOSE_PAYLOAD_LOGGING` | `false` | When `true`, additionally logs the rewritten request body at `debug`. Local debugging only — never enable this against real traffic. |

The inbound `Authorization` header, if present, is forwarded to
`VLLM_BASE_URL` unchanged; the proxy holds no separately configured
upstream credential.

## Endpoints

- `POST /v1/chat/completions` — the proxy's only real route. Coerces
  mid-conversation system messages, forwards to vLLM, and streams the
  response back without buffering it.
- `GET /health` — returns `200 ok`, independent of any upstream connectivity.
- `GET /metrics` — Prometheus exposition format. See **Metrics** below.

## Error responses

Errors on the request path never return vLLM's raw error text, a stack
trace, or an axum default rejection body — always this shape:

```json
{"error": {"message": "...", "type": "invalid_request | upstream_error | internal_error"}}
```

| Status | Cause | `type` |
|---|---|---|
| 400 | Malformed JSON, not an object, or missing `messages` array | `invalid_request` |
| 413 | Body exceeds `MAX_REQUEST_BODY_SIZE` | `invalid_request` |
| 502 | Failed to connect to `VLLM_BASE_URL` | `upstream_error` |
| 504 | Upstream didn't respond within `UPSTREAM_TOTAL_TIMEOUT_SECS` | `upstream_error` |
| 500 | Unexpected internal failure | `internal_error` |

A non-2xx response *from* vLLM itself is not one of these — it is still
passed through to the client verbatim, exactly as vLLM sent it (only its
`kind="non_2xx"` count is recorded in `proxy_upstream_errors_total`).

## Logging

Structured via `tracing`, filtered by `LOG_LEVEL`. A coercing request logs
the count and original indices of every coerced message at `debug` — never
message content, unless `VERBOSE_PAYLOAD_LOGGING=true`. Upstream failures
log their kind at `error`.

## Metrics

`GET /metrics`, Prometheus text format:

| Metric | Type | Labels |
|---|---|---|
| `proxy_requests_total` | Counter | `status` (HTTP status code) |
| `proxy_request_duration_seconds` | Histogram | — |
| `proxy_messages_coerced_total` | Counter | — |
| `proxy_upstream_errors_total` | Counter | `kind` (`connect_timeout`, `request_timeout`, `non_2xx`) |

## Graceful shutdown

On SIGINT (Ctrl+C) or SIGTERM, the server stops accepting new connections
and waits for in-flight requests — including long-lived streaming ones — to
finish before exiting.

## Development

```bash
cargo test --all-targets            # full suite
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo audit                         # RUSTSEC advisory scan against Cargo.lock
cargo bench --bench transform_bench # criterion benchmark; see Performance below
```

`unwrap()`, `expect()`, and `panic!()` are denied crate-wide via
`[lints.clippy]` in `Cargo.toml` (test code and benchmark-fixture code are
explicitly `#[allow]`ed where that's normal).

## Performance

### Design: targeted mutation, not a full owned rebuild

`transform.rs`'s core function, `coerce_messages`, is a pure function over a
mutable slice of `serde_json::Value` messages. It never deserializes into a
struct-per-field model; it walks the array once and mutates only the `role`
and `content` fields of messages that actually need coercion (index > 0,
`role == "system"`). Every other message — and every other field on a
coerced message (`tool_calls`, `tool_call_id`, `name`, cache-control hints,
unknown fields) — is left completely untouched: no clone, no rebuild.
Content prefixing allocates exactly one right-sized `String` per rewritten
message (`String::with_capacity`), rather than an intermediate prefix string
plus a `format!`/`+` concatenation.

### The dual-dispatch parser, and what the benchmark found

The approved design calls for runtime dispatch between `simd-json`
(preferred, when this CPU/architecture supports it) and `serde_json`
(fallback) for the parse step. That dispatch is implemented as specified —
`simd_json::serde::from_slice` is a serde-generic deserializer, so it can
target `serde_json::Value` directly, meaning both parse paths share this
crate's one mutation implementation rather than needing a duplicated
algorithm per value-tree type (see the module doc in `src/transform.rs`).

`benches/transform_bench.rs` measures four variants at three message-array
sizes (10/100/1000) — the production simd-json-dispatch path, the same
targeted-mutation core forced through `serde_json` only, a full-rebuild
baseline using the same `Value` representation, and a full-rebuild baseline
using typed structs — plus a one-shot allocation-count comparison via a
`#[global_allocator]` counter (std-only, no extra dependency). On this
development machine (aarch64, `cargo bench -- --sample-size 10
--measurement-time 1`), at 100 messages:

| Variant | Latency | Allocations |
|---|---|---|
| Targeted mutation, simd-json dispatch (production) | ~70 µs | 698 |
| Targeted mutation, serde_json only | ~72 µs | 690 |
| Full rebuild via `Value` (same representation) | ~102 µs | 1319 |
| Full rebuild via typed structs | ~42 µs | 480 |

Two findings worth being explicit about:

1. **Targeted mutation is a real win** when holding the value
   representation constant: ~70µs/698 allocations vs. ~102µs/1319
   allocations against a full-rebuild-via-`Value` baseline — confirming the
   core design decision (touch only what needs to change).
2. **The simd-json dispatch shows no clear win over serde_json alone** at
   this workload size on this machine, and costs a handful of extra
   allocations — simd-json's in-place parser requires an owned, mutable
   buffer (`body.to_vec()`), a copy the serde_json path doesn't pay, and its
   serde-generic bridge (needed to target `serde_json::Value` rather than
   its own native `OwnedValue`) has its own overhead that can offset its
   parsing speed advantage for payloads in this size range. The dispatch is
   still implemented and kept — it is correct, tested, and was the
   explicitly human-approved design, and simd-json's advantage may be more
   pronounced on other architectures or much larger payloads — but a future
   iteration might reasonably revisit whether it earns its complexity here.

### Why not a fully-typed struct model?

The full-rebuild-via-typed-structs baseline is faster in this benchmark than
either `Value`-based path. It is not, however, a fair comparison on
generality: the benchmark's naive struct (`NaiveMessage { role, content }`)
does not attempt to round-trip arbitrary unknown message fields
(`tool_calls`, `tool_call_id`, `cache_control`, forward-compatible fields a
future API version might add), which was an explicit requirement for the
real implementation ("round-trip unknown fields, do not assume a closed
schema"). A fully general typed-struct model would need a
`#[serde(flatten)] extra: Map<String, Value>` catch-all per message, which
narrows but does not necessarily close this gap. `serde_json::Value`'s
one-full-parse cost — the "if a full round-trip is unavoidable, document
why" case named in the original task — is the price of handling an open
schema; see the module doc in `src/transform.rs` for the complete accounting
of what is and isn't zero-copy in the final design.

### Full-round-trip caveat

Re-serializing a `serde_json::Value` produces a canonical minimal JSON
rendering. Untouched *field values* round-trip exactly, and this crate
enables serde_json's `preserve_order` feature so object key order is
preserved too — but insignificant input formatting JSON does not preserve
semantically (extra whitespace, non-canonical number formatting like `1.50`
vs `1.5`) is not reproduced. This is the one full parse's unavoidable cost
under `serde_json::Value`'s ownership model.

## Design documents

The full requirements, domain design, NFR requirements/design, practices,
and code-generation plan this implementation follows live under
`aidlc/spaces/default/intents/260905-role-coercion-proxy/`.
