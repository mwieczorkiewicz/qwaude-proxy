# Code Generation Plan — qwaude-proxy (role-coercion-proxy)

Zero-Unit, stage-level plan (no Units Generation ran — single-binary project). Every step maps back to an intent-backlog item (`ideation/scope-definition/intent-backlog.md`, FR1-FR13) and/or an NFR sub-requirement.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "custom",
  "source": "team",
  "ordering": "write property-based tests before or alongside the",
  "scope": "qwaude-proxy",
  "test_strategy": "standard",
  "project_type": "greenfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "We treat tests as a first-class deliverable in every Bolt. The specific\nmethodology (TDD, BDD, ATDD, or classic test-after) is affirmed at\npractices-discovery and recorded in `team.md` under this heading with explicit\n`Methodology` and `Ordering` fields; Code Generation resolves those fields\nindependently from coverage, tooling, and scope notes.\n\nWhen no posture has been affirmed, our default per scope is:\n- **Methodology**: test-after\n- **Ordering**: implement each applicable testable layer, then write and run\n  that layer's tests.\n- `mvp`, `enterprise`, `feature`, `infra`, `classic` add an 80% line-coverage\n  floor and CI execution before merge.\n- `bugfix`, `security-patch` add a targeted regression for the specific\n  bug/vulnerability and require the existing suite to remain green.\n- `express` uses the Minimal strategy: requirement-driven unit tests (one per\n  requirement, with a happy-path floor per component); existing tests remain\n  green.\n- `poc`, `refactor`, `workshop` add no extra new-test floor and require the\n  existing suite to remain green.\n\nThe active `Test Strategy` still applies in every scope and determines test\nvolume/types. Scope floors are additive; they never reduce or replace the\nselected strategy.\n\nBuild and Test verifies defined coverage floors and affirmed quality targets;\nthey may not be weakened to make a step pass.\n\nAffirm a stricter posture in `team.md` if the team commits to one."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: custom\n- **Ordering**: write property-based tests before or alongside the\n  implementation for `transform.rs` specifically (tests-first for that\n  module only), and for every other layer — HTTP wiring, configuration,\n  logging, shutdown, error handling — implement the layer first and then\n  write and run that layer's tests (test-after), per the confirmed interview\n  answer (Q3).\n\n`custom` is used here per this stage's own rule for a mixed-cadence answer:\n`transform.rs` is tests-first, everything else is test-after — a single\nuniform label would misrepresent the confirmed decision.\n\nCoverage, tooling, test types, and CI scope (Q4, Q5):\n\n- **Coverage target/tooling**: aim for 80% coverage (line + branch, measured\n  via `cargo-llvm-cov`), but this is a **target, not a hard gate** — a module\n  falling short of 80% is acceptable case-by-case if the test suite's\n  completeness (happy path + edge cases + invariants covered) is otherwise\n  satisfactory. `cargo-llvm-cov` is used in preference to `cargo-tarpaulin`\n  for better accuracy against this project's async (`tokio`-driven) code.\n- **Test types**: unit tests, property-based tests (proptest/quickcheck) for\n  `transform.rs`, integration tests against a mock upstream capable of\n  streaming chunked responses (not just a canned body), and a criterion\n  benchmark comparing the zero-copy transform against a naive\n  full-deserialize baseline.\n- **Benchmark is tracked/reported, not a hard merge gate.** `cargo test`,\n  `cargo clippy -D warnings`, and `cargo fmt --check` passing clean is the\n  merge bar; the criterion benchmark result is not required to show zero\n  regression to merge, since bench numbers are inherently noisy across\n  machines.\n- **CI scope**: \"deployment/infrastructure/CI-CD automation is out of scope\"\n  refers to deployment/hosting automation only. A CI workflow that runs\n  `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, and\n  `cargo audit` on every push/PR — with no deploy step — is in-scope project\n  tooling, not deployment infrastructure (Q5). These checks are therefore\n  build-enforced by that workflow, not left to manual discipline alone.\n- **Streaming backpressure** (\"streams SSE responses through without full\n  buffering, preserving backpressure\") is a specific, testable claim and\n  needs an explicit test case (a slow consumer against a fast/large upstream\n  stream, asserting the proxy never buffers the full body) rather than being\n  left implicit inside a generic \"integration tests\" bucket.\n- **`transform.rs` invariants to name explicitly** as property tests:\n  `messages[0]` with `role: \"system\"` is never mutated; every other `system`\n  message becomes `role: \"user\"` with the notice prefix prepended; every\n  other field (`tool_calls`, `tool_call_id`, `name`, cache-control hints,\n  unknown fields) round-trips byte-identical; both string-content and\n  content-block-array content shapes are handled with equivalent effect.\n- `proptest`'s failing-case seed files (`proptest-regressions/<module>.txt`)\n  are committed to the repo, not gitignored, so a once-found counterexample\n  can never silently regress."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "custom",
    "runner_step": "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Bootstrap the minimal test runner/configuration and record the exact unit-scoped command.",
      "Custom ordering - write property-based tests before or alongside the",
      "Implementation and tests - preserve that exact ordering; do not convert it to layer-local TDD.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:a6d843ee708a585e8ffde6820fe6f9197fe8e8ea5ac6fa9eb975a58b106204f0",
  "contract_sha256": "sha256:957538ca317a46a279f3a797181ca7ae518edb86aa2aa61a7cc4c134a403ca9f"
}
```

## Plan Steps

- [ ] **Step 1 — Project structure and production configuration skeleton.**
  `Cargo.toml` (pinned dependencies: axum, hyper/hyper-util, tokio, serde/serde_json, simd-json, tracing/tracing-subscriber, thiserror, metrics/metrics-exporter-prometheus; dev-dependencies: proptest, criterion, wiremock, tokio-test); `src/main.rs`, `src/transform.rs`, `src/config.rs`, `src/error.rs`, `src/metrics.rs` as empty/stub modules; `rustfmt.toml`; `.gitignore`; `benches/` and `tests/` directories with `tests/common/mod.rs` stub.
  *Backlog:* scaffolding for all of FR1-FR13.

- [ ] **Step 2 — Bootstrap the test runner.**
  Confirm `cargo test --lib` runs (even trivially) before any test-first work begins; record the exact command in `unit-test-instructions.md`.
  *Backlog:* FR9 (test infrastructure).

- [ ] **Step 3 — `transform.rs` tests FIRST (property + unit).**
  Per the custom ordering, write these BEFORE any `Transform` implementation exists (RED):
  - Property tests (proptest): for any valid input message array, the output (a) has the same length, (b) has no `role: "system"` at any index other than 0, (c) preserves relative order/content of non-rewritten fields.
  - Unit tests: leading system message left alone; single mid-stream system message coerced+prefixed; multiple mid-stream system messages; string-content shape; content-block-array shape (only first text block prefixed); message with no `content` field / non-standard shape does not panic; unknown/extra fields round-trip unchanged.
  *Backlog:* FR1, FR9.

- [ ] **Step 4 — Implement `transform.rs` (GREEN).**
  `TransformError` type; the role-coercion logic (leave `messages[0]` alone if system; every other system message → role `"user"` + prefixed content; both content shapes; unknown-field round-trip); runtime SIMD-feature detection dispatching between `simd-json` and `serde_json` (per `nfr-design/performance-design.md`); coercion-count/index tracking for the caller to log. Make Step 3's tests pass.
  *Backlog:* FR1; NFR1.1, NFR1.2, NFR1.3.

- [ ] **Step 5 — Implement `config.rs`, then test.**
  `ProxyConfig`: `LISTEN_ADDR` (default `0.0.0.0:8080`), `VLLM_BASE_URL` (default `http://127.0.0.1:8000`), `NOTICE_PREFIX` (default `[System Notification] `), `LOG_LEVEL` (default `info`), `MAX_REQUEST_BODY_SIZE` (default 10 MiB), connect/total upstream timeouts (defaults 5s/30s) — all env-var-overridable per `nfr-design/security-design.md` NFR2.7 and `reliability-design.md` NFR4.1. Unit tests: each var's default and override.
  *Backlog:* FR5; NFR2.7, NFR4.1.

- [ ] **Step 6 — Implement `error.rs`, then test.**
  `ProxyError` enum composing `TransformError` plus upstream-forwarding failures; `IntoResponse` impl producing the confirmed `{"error": {"message", "type"}}` shape (never raw upstream text or a stack trace), mapping to 400 (malformed input)/413 (body too large)/502 (upstream connection failure)/504 (upstream timeout). Unit tests: one per variant, asserting status code + body shape.
  *Backlog:* FR8; NFR2.6.

- [ ] **Step 7 — Implement `main.rs` HTTP server, then integration-test it.**
  `POST /v1/chat/completions` handler wrapped in `DefaultBodyLimit` (Step 5's limit); invokes `Transform`; forwards to `VLLM_BASE_URL` via the `hyper`-based client, passing the inbound `Authorization` header unchanged, applying Step 5's timeouts (streaming responses only bounded until first byte); streams SSE/chunked responses through via `axum::body::Body::from_stream` with no full-body buffering; passes non-streaming responses through verbatim; `GET /health`; graceful-shutdown wiring stub (completed in Step 9). Integration tests (`wiremock`): non-streaming round-trip; streaming SSE pass-through (assert bytes arrive before the mock upstream finishes sending); large message arrays; malformed JSON → 400; upstream connection refused → 502; oversized body → 413; `Authorization` header forwarded unchanged.
  *Backlog:* FR2, FR3, FR4, FR8, FR10; NFR1.4, NFR1.5, NFR2.1, NFR2.7, NFR4.1, NFR5.2.

- [ ] **Step 8 — Implement structured logging, then verify.**
  `tracing_subscriber` + `EnvFilter` from `LOG_LEVEL`; per-request span; `debug!` with coercion count/indices; `error!` with upstream failure kind; no payload content unless the verbose-logging flag is set. Test: capture emitted log fields for a coercing and a non-coercing request, assert no payload content leaks by default.
  *Backlog:* FR6; NFR5.1.

- [ ] **Step 9 — Implement graceful shutdown, then test.**
  `axum::serve(...).with_graceful_shutdown(...)` fed by a future racing `tokio::signal::ctrl_c()` and SIGTERM; drains in-flight requests including streaming ones. Integration test: start a slow/streaming mock request, send the shutdown signal, assert the in-flight response completes before the server exits.
  *Backlog:* FR7; NFR4.2.

- [ ] **Step 10 — Implement metrics, then test.**
  `metrics` + `metrics-exporter-prometheus` recorder installed at startup; `GET /metrics` route; four metrics (`proxy_requests_total{status}`, `proxy_request_duration_seconds`, `proxy_messages_coerced_total`, `proxy_upstream_errors_total{kind}`). Test: `GET /metrics` after a few requests contains the expected metric names.
  *Backlog:* NFR5.3.

- [ ] **Step 11 — Environment/build configuration: lint and format gates.**
  `[lints.clippy]` table in `Cargo.toml` enabling `unwrap_used`, `expect_used`, `panic` (scoped at minimum to `main.rs`'s handler and `transform.rs`, narrow `#[allow(...)]` in `#[cfg(test)]` modules); confirm `cargo clippy -- -D warnings` and `cargo fmt --check` run clean locally.
  *Backlog:* NFR2.5 (project `Mandated` rule).

- [ ] **Step 12 — Criterion benchmark.**
  `benches/transform_bench.rs` (`harness = false` in `Cargo.toml`) comparing the zero-copy transform path against a naive full-deserialize/full-rebuild/full-reserialize baseline — latency and, where measurable, allocation count. Document any place a true zero-copy implementation wasn't possible and its actual allocation cost, per the original task's explicit request.
  *Backlog:* FR11; NFR1.1, NFR1.2, NFR1.3.

- [ ] **Step 13 — Documentation and traceability.**
  `README.md` (problem, pipeline position, configuration, how to run tests/benchmarks); finalize `code-summary.md` and `traceability.json`.
  *Backlog:* FR13.

## Story-to-Code-Step Traceability

| Backlog Item | Plan Step(s) |
|---|---|
| FR1 (transform logic) | Steps 3, 4 |
| FR2 (HTTP server + forwarding) | Step 7 |
| FR3 (streaming pass-through) | Step 7 |
| FR4 (non-streaming pass-through) | Step 7 |
| FR5 (configuration) | Step 5 |
| FR6 (structured logging) | Step 8 |
| FR7 (graceful shutdown) | Step 9 |
| FR8 (error handling) | Steps 6, 7 |
| FR9 (unit + property tests) | Steps 2, 3 |
| FR10 (integration tests) | Step 7 |
| FR11 (criterion benchmark) | Step 12 |
| FR12 (clippy/fmt clean) | Step 11 |
| FR13 (README) | Step 13 |
| NFR1.1-1.5 (performance) | Steps 4, 7, 12 |
| NFR2.1-2.7 (security) | Steps 5, 6, 7 |
| NFR3.1-3.4 (scalability) | No dedicated step — a design property preserved throughout (no shared mutable state introduced anywhere) |
| NFR4.1-4.4 (reliability) | Steps 5, 7, 9 |
| NFR5.1-5.4 (observability) | Steps 8, 9 (health), 10 |

## Plan Approval

[Approval Fingerprint]: sha256:e09cec1e21afeeb853aa3c9486ae1df7c4cbf366f500d7fcab3a71e2c94f2212

- Approve Plan
- Request Changes

[Answer]:
