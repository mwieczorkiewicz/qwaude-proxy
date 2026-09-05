# Observability Design — qwaude-proxy (role-coercion-proxy)

## NFR5.1 — Structured logging

**Design**: `tracing_subscriber` is initialized at startup with an `EnvFilter` driven by the `LOG_LEVEL` environment variable. Each incoming request opens a `tracing` span carrying the method and path (no payload content); within that span:
- A `debug!` event fires after the transform runs, carrying the coerced-message count and their original indices as structured fields.
- An `error!` event fires on any upstream failure, carrying the failure kind (connect timeout, request timeout, non-2xx status) as a structured field — never the raw upstream response body.
- Payload content is included only when the verbose-payload-logging config flag is explicitly enabled, gated at the log-call site (a conditional check before the field is populated, not just a filtered-out log level) so the content is never even formatted into a string when the flag is off.

## NFR5.2 — Health endpoint

**Design**: `GET /health` is a plain axum route handler returning `200 OK` with a minimal body (e.g. `"ok"`), registered independently of the `POST /v1/chat/completions` route's body-limit and forwarding logic — it never touches `Transform`, the outbound client, or any configuration beyond confirming the process is serving requests.

## NFR5.3 — Metrics endpoint

**Design**: the `metrics` facade crate records four metrics, exported in Prometheus text format at `GET /metrics` via `metrics-exporter-prometheus`'s installed recorder:

| Metric | Type | Labels | Purpose |
|---|---|---|---|
| `proxy_requests_total` | Counter | `status` (HTTP status code) | Request volume by outcome |
| `proxy_request_duration_seconds` | Histogram | — | Latency distribution (validates NFR1.1's p99 target) |
| `proxy_messages_coerced_total` | Counter | — | Cumulative count of role-coerced messages (operational visibility into how often the original bug's trigger condition actually fires) |
| `proxy_upstream_errors_total` | Counter | `kind` (connect_timeout, request_timeout, non_2xx) | Upstream failure volume by cause |

## NFR5.4 — No distributed tracing

Not applicable — restated from `observability-requirements.md`; single-hop proxy, no multi-service trace propagation needed.

## Support Perspective (AWS Platform)

No CloudWatch, X-Ray, or managed observability service integration applies — metrics and logs are exposed via standard endpoints (`/metrics`, stdout via `tracing`) for whatever scraper or log collector a future deployment chooses, rather than a specific AWS service, since deployment/infrastructure is out of scope for this workflow.

## Source

`observability-requirements.md` (NFR5.1-5.4).

## Assumptions & Open Questions

None.
