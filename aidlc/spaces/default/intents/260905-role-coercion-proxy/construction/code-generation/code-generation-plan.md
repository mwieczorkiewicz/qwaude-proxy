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

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-06T14:56:37Z
**Iteration:** 1

This is an ADVISORY pass: findings below are ranked for the human to weigh at the approval gate, not a blocking gate.

### Findings

#### Critical

None.

#### Major

None.

#### Minor

1. **`Limited::collect()` failures are all mapped to `413 Payload Too Large`, including non-size-related I/O errors.** `server.rs::handle_chat_completions` does `limited_body.collect().await.map_err(|_| ProxyError::BodyTooLarge)?`. `http_body_util::Limited`'s error variant covers both "limit exceeded" and the underlying body stream erroring (e.g. the client disconnecting mid-upload, a malformed chunked transfer-encoding frame). Both collapse to the same 413 response with no `tracing::error!` call — unlike every other error path in this file (upstream-connect-failed, upstream-timeout both log before converting). This is low-impact (the client that triggered a genuine transport error is usually already gone, and the wrong status code on a disconnected client is harmless in practice) but it does mean a real body-transport fault is silently mislabeled as "body too large" in any logs/metrics that do capture it, which could mislead someone debugging a spike in 413s that isn't actually about oversized requests.
   - *Location*: `src/server.rs` > `handle_chat_completions`, the `collect().await.map_err(...)` line.
   - *Suggested action (non-blocking)*: distinguish `Limited`'s "over the limit" variant from a genuine body-stream error (e.g. via `Limited`'s `LengthLimitError`/inner-error downcast), map the latter to `ProxyError::Internal` or a new variant, and log it — or explicitly note in `code-summary.md` that this collapse is an accepted simplification.

2. **`traceability.json` reuses the undocumented `"N/A"` coverage status** (NFR3.3, NFR3.4, NFR4.4, NFR5.4) that the Domain Design review (R-03, `components.md`) already flagged as an extension beyond the stage's documented `OK`/`GAP` vocabulary. The reasoning behind each `N/A` here is sound and consistent with the same IDs' `N/A` disposition all the way back through `nfr-requirements` and `nfr-design` (and the `nfr-design` review's Cross-Artifact Consistency Checks note this pattern passed the `traceability` sensor at that stage without incident), so this is very unlikely to be a live problem — but it's the same open item, now propagated into a third artifact, and still not explicitly confirmed as tolerated by the `traceability` sensor's actual validation logic.
   - *Location*: `traceability.json` > `coverage[]` entries for `NFR3.3`, `NFR3.4`, `NFR4.4`, `NFR5.4`.
   - *Suggested action (non-blocking)*: no action needed if the `traceability` sensor is confirmed to already accept `N/A` (which its clean pass at nfr-design suggests); otherwise fold these into an `OK`-with-note convention.

3. **`components.md`'s R-01 finding (Transform's failure modes left implicit) is resolved in the actual code but the resolution isn't cross-referenced anywhere in this stage's artifacts.** The Domain Design review flagged that `Transform`'s `behaviour` never stated whether/how it could fail. The generated code answers this cleanly — `TransformError` (`InvalidJson`/`NotAnObject`/`MissingMessagesArray`) is well-scoped and propagates through `ProxyError::InvalidRequest` to a 400 — but neither `code-summary.md` nor `traceability.json` explicitly notes that this closes the earlier architecture-review gap, so a reader auditing review-finding resolution across stages has to independently reconstruct that connection.
   - *Location*: `code-generation/code-summary.md` (no cross-reference); compare `inception/domain-design/components.md` Review > R-01.
   - *Suggested action (non-blocking)*: informational only — no artifact change required for this to be sound engineering, just a traceability nicety.

### Verification Performed

- **`transform.rs` role-coercion logic**: read in full. `messages[0]` is unconditionally skipped (`if idx == 0 { continue; }`) regardless of role, matching the domain design's "leave index 0 alone" intent (the code's own doc comment notes this is stricter than "leave alone only if system," but produces identical externally observable behavior since index 0 is never a coercion candidate either way). Every other `role: "system"` message is rewritten to `"user"` with `notice_prefix` prepended. Both `string` and content-block-array `content` shapes are handled: string is prefixed directly; array finds the first block with `"type": "text"` and prefixes only that block's `"text"` field, leaving every other block (images, later text blocks, cache-control hints) untouched. All other message fields (`tool_calls`, `tool_call_id`, `name`, `cache_control`, unknown fields) round-trip untouched because only the `role` key is overwritten and only the targeted `content`/nested `text` value is mutated — verified against the `unknown_and_extra_fields_round_trip_unchanged` unit test and the `prop_non_coerced_messages_are_untouched` property test, both present and asserting the right things. Non-standard content (`null`, a number, missing) does not panic — verified against `message_with_missing_or_non_standard_content_does_not_panic`. `serde_json`'s `preserve_order` feature is enabled specifically to keep field order stable, which is a correct and non-obvious detail to have gotten right.
- **Zero-copy/targeted-mutation claim**: verified honest, not just claimed. The module doc, `code-summary.md`, and `README.md`'s Performance section all consistently state that `serde_json::Value` has no borrowed mode, so one full parse is unavoidable, and that the actual "zero-copy" property is the *targeted-mutation* discipline after that parse (only coerced messages' `role`/`content` are touched). The benchmark (`benches/transform_bench.rs`) backs this with measured allocation counts (698 vs. 1319 for a full `Value` rebuild at 100 messages) rather than an unmeasured assertion, and further honestly reports that the simd-json dispatch path shows no clear latency win over `serde_json` alone on the development machine and costs extra allocations — a finding that undercuts the design's own premise, reported rather than suppressed. This is a stronger-than-typical honesty bar for a generated summary and it holds up against the code.
- **No-panic/no-unwrap requirement**: `grep -n -E '\.unwrap\(\)|\.expect\(|panic!' src/*.rs` finds 16 matches, every one inside a `#[cfg(test)] mod tests` block (confirmed by reading each file's structure) — none on the request path. `Cargo.toml`'s `[lints.clippy]` denies `unwrap_used`/`expect_used`/`panic` crate-wide (broader than the team's "at minimum" scoping requirement), with narrow `#[allow(...)]` at the top of test/bench files, matching the affirmed convention.
- **Streaming pass-through**: verified real, not just claimed. `server.rs::handle_chat_completions` wraps the upstream `hyper` response body directly (`Body::new(upstream_body)`) with no intermediate `.collect()`/buffering step — the same code path handles both SSE and ordinary chunked responses. `tests/integration.rs::streaming_response_is_forwarded_incrementally_not_buffered` backs this with a genuine timing assertion against a hand-rolled chunked mock upstream (`tests/common/mod.rs::spawn_chunked_upstream`, built because `wiremock` cannot simulate an incrementally-arriving response) — it asserts the first byte arrives well before the time it takes the mock to finish sending all chunks, which is a real backpressure/non-buffering proof, not a superficial check. `tests/shutdown.rs` reuses the same mock to verify an in-flight streaming response is drained (not aborted) across a graceful-shutdown signal.
- **Error handling**: verified every error path in `error.rs`'s `IntoResponse` impl constructs the confirmed `{"error": {"message", "type"}}` shape via a single `ErrorBody`/`ErrorDetail` struct pair, the only construction site in the crate. Status codes match the confirmed vocabulary: `InvalidRequest` (from `TransformError`) → 400, `BodyTooLarge` → 413, `UpstreamConnectFailed` → 502, `UpstreamTimeout` → 504 (plus a 500 `Internal` catch-all outside the four named codes, a reasonable addition for truly unexpected failures like outbound-request-construction errors). `public_message()` returns fixed, generic strings per variant — never the real upstream error text or a `Debug`/stack-trace rendering — confirmed by reading every variant's message and by the `internal_error_maps_to_500_and_never_leaks_detail` unit test. Every one of the four confirmed codes has a matching `tests/integration.rs` integration test asserting both status and body shape. One residual gap noted above (Minor #1): `Limited` collect errors other than "over limit" are folded into 413 without distinction or logging.
- **The two documented deviations**: both verified as genuine engineering-judgment calls, not silent behavior changes. (1) `http_body_util::Limited` used directly instead of the `DefaultBodyLimit` tower layer — read `server.rs`'s module doc and confirmed the reasoning (the handler takes the raw `Request` to run the byte-level `coerce_system_messages`, so it can't use the `Bytes`/`Json` extractors `DefaultBodyLimit` targets); the resulting 413 behavior (same limit, same status code, same JSON error shape) is unchanged from what the plan intended. (2) The two extra modules (`lib.rs`, `server.rs`) — confirmed as the standard idiomatic `lib.rs`+thin-`main.rs` split needed for `tests`/`benches` to build the router in-process, and `server.rs` is exactly the HTTP-routing/forwarding code the plan's Step 7 already described without naming a file. Neither deviation changes externally observable behavior from what was approved.
- **`traceability.json` coverage spot-checks**: `FR1` → `src/transform.rs` (confirmed, the whole file is the coercion logic). `NFR1.5` → `src/server.rs` (`Body::new(upstream_body)`) + `tests/integration.rs` + `tests/shutdown.rs` (confirmed both files contain streaming assertions, not just generic integration tests). `NFR2.7` → `src/config.rs` (`MAX_REQUEST_BODY_SIZE`/`DEFAULT_MAX_REQUEST_BODY_SIZE`) + `src/server.rs` (`Limited::new`) + `tests/integration.rs` (`oversized_body_returns_413`, confirmed present and correctly asserting 413 + `invalid_request` type). `NFR2.1` → `src/server.rs` (`Authorization` header cloned unchanged) + `tests/integration.rs::authorization_header_is_forwarded_unchanged` (confirmed, asserts via a `wiremock` matcher on the forwarded header rather than just trusting the code). `FR12` → `Cargo.toml` `[lints.clippy]` + `.github/workflows/ci.yml` (confirmed both exist with the claimed content). All spot-checked `OK` targets are genuine, existing files containing what they claim.
- **Upstream design consistency**: cross-checked against `components.md` (Transform/ProxyServer boundary respected — `transform.rs` remains pure/I/O-free, `server.rs` owns all I/O), `performance-design.md` (dual simd-json/serde_json dispatch implemented as specified, including the `OnceLock`-cached CPU-feature check), `security-design.md` (the `ErrorResponse`/`ErrorDetail` shape and `type` enum values — `invalid_request`/`upstream_error`/`internal_error` — match exactly), and `reliability-design.md` (5s connect / 30s total timeout defaults, streaming-response first-byte-only bound, graceful SIGINT/SIGTERM drain all implemented as designed). No unflagged divergence found between the approved design and the generated code.

### Validation Tool Results

No `validation_tools` field is declared in this stage's frontmatter; none were run programmatically by this reviewer. `cargo test`/`cargo clippy`/`cargo fmt`/`cargo audit`/`cargo llvm-cov` results are as self-reported in `code-summary.md` (47/47 tests passing, clippy/fmt clean, 0 audit advisories, 86.19% line coverage) — this reviewer did not independently re-execute the build/test suite, per the read-only nature of this pass, but the claimed test files and assertions were read directly and confirmed to test what they claim to test (see Verification Performed above), which is stronger evidence than the summary numbers alone.

### Summary

The generated code is a faithful, well-tested implementation of the approved plan and upstream design: the transform logic correctly handles every named edge case (leading system message, mid-stream coercion, both content shapes, non-standard/missing content, unknown-field round-tripping) with both unit and property-test coverage backing each claim; streaming pass-through is real and proven by a genuine backpressure-timing test against a hand-rolled mock upstream, not merely asserted; every error path returns the confirmed sanitized JSON shape with the correct status code; the no-panic/no-unwrap requirement is mechanically enforced and verified clean outside test code; and the zero-copy design's stated limitations are honestly measured and reported, including a finding (simd-json's unclear benefit) that cuts against the design's own premise rather than being suppressed. The two documented deviations are genuine engineering judgment calls that preserve identical external behavior. The three findings above are all minor and non-blocking: an imprecise error-mapping edge case in body-size-limit handling, a carried-forward (and very likely harmless) traceability-schema question about the `"N/A"` status value, and a missed opportunity to cross-reference how this stage's code resolves an earlier review finding.
