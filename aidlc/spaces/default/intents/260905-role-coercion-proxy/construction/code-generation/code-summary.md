# Code Generation Summary — role-coercion-proxy

All 13 steps of `code-generation-plan.md` are complete. This is a
zero-Unit, stage-level dispatch — the entire single-binary crate was built
in this one pass, at the workspace root
`/Users/mikolajwieczorkiewicz/software-engineering/qwaude-proxy/`.

## Files created

Application source (also listed in `source-manifest.json`):

- `Cargo.toml`, `Cargo.lock` — dependency manifest and lockfile (committed, per team practice, for `cargo audit` and reproducible builds).
- `rustfmt.toml`, `.gitignore` (extended with a Rust `/target/` entry).
- `.github/workflows/ci.yml` — fmt/clippy/test + a separate `cargo audit` job, no deploy step.
- `src/main.rs` — thin binary entry point: resolve config, init logging, bind, serve with graceful shutdown.
- `src/lib.rs` — re-exports the library modules so `tests/`/`benches/` can build the router and call `transform` in-process.
- `src/config.rs` — `ProxyConfig`, env-var resolution with documented defaults.
- `src/error.rs` — `ProxyError`, the single `IntoResponse` construction site.
- `src/transform.rs` — the pure role-coercion logic; the module the team's tests-first ordering applies to.
- `src/server.rs` — HTTP routing, forwarding, streaming pass-through, metrics recording.
- `src/logging.rs` — `tracing_subscriber` setup from `LOG_LEVEL`.
- `src/metrics.rs` — Prometheus recorder installation and metric name constants.
- `src/shutdown.rs` — the SIGINT/SIGTERM future fed to `axum::serve(...).with_graceful_shutdown(...)`.
- `tests/common/mod.rs` — shared test router builder plus a hand-rolled chunked-response mock upstream.
- `tests/integration.rs`, `tests/logging.rs`, `tests/shutdown.rs`, `tests/metrics.rs` — integration suites.
- `benches/transform_bench.rs` — the criterion benchmark (Step 12).
- `README.md` — problem statement, configuration reference, error/log/metric shapes, Performance section.

## Plan steps — status

| Step | Status |
|---|---|
| 1. Project structure and production configuration skeleton | Done |
| 2. Bootstrap the test runner (`cargo test --lib --no-run`) | Done — confirmed clean before any transform.rs test existed |
| 3. `transform.rs` tests FIRST (property + unit) | Done — 14 real tests committed RED (`unimplemented!()` stub), separately from Step 4 |
| 4. Implement `transform.rs` (GREEN) | Done — all 15 tests (14 + 1 trivial) pass; dual simd-json/serde_json dispatch implemented |
| 5. Implement `config.rs`, then test | Done — 9 unit tests |
| 6. Implement `error.rs`, then test | Done — 7 unit tests |
| 7. Implement `main.rs`/`server.rs` HTTP server, then integration-test | Done — 8 integration tests |
| 8. Implement structured logging, then verify | Done — 3 logging tests with a scoped (non-global) subscriber |
| 9. Implement graceful shutdown, then test | Done — 1 real-server drain test |
| 10. Implement metrics, then test | Done — 2 metrics tests |
| 11. Environment/build configuration: lint and format gates | Done — clippy/fmt/audit all clean; CI workflow added |
| 12. Criterion benchmark | Done — 4-way latency/allocation comparison |
| 13. Documentation and traceability | Done — this file, README.md, source-manifest.json, traceability.json |

## Key implementation decisions

### 1. `transform.rs`'s API split

`coerce_messages(messages: &mut [Value], notice_prefix: &str) -> CoercionReport`
is the pure core (directly unit/property-testable on a `Vec<Value>`, no I/O),
and `coerce_system_messages(body: &[u8], notice_prefix: &str) -> Result<(Vec<u8>, CoercionReport), TransformError>`
is the byte-level entry point `server.rs`'s handler calls. This split let the
Step 3 tests target the algorithm directly rather than needing a wrapping
JSON object literal per test case.

### 2. simd-json/serde_json dual dispatch — implemented as approved, with a documented caveat

The initial attempt at this dispatch (before the human's mid-turn correction)
incorrectly assumed `simd_json`'s and `serde_json`'s `Value` types were
structurally incompatible and would force a duplicated mutation algorithm.
That's wrong: `simd_json::serde::from_slice` is a serde-*generic*
deserializer and can target `serde_json::Value` directly, so both parse
paths share this crate's one `coerce_messages` implementation.
`simd_json_available()` checks CPU/architecture support once per process
(`OnceLock`); `parse_via_simd_json`/`parse_via_serde_json` are split into
independently-callable, independently-tested functions.

**Benchmark finding worth flagging** (see `benches/transform_bench.rs` and
README's Performance section for the full numbers): on this development
machine (aarch64), the simd-json path shows no clear latency win over
serde_json alone at typical chat-completion message-array sizes, and costs
a handful of extra allocations from the owned-buffer copy simd-json's
in-place parser requires. The dispatch is correct, tested, and was the
explicitly human-approved design, so it's kept as implemented — but this is
worth revisiting in a future iteration with real production traffic
patterns and on the actual deployment architecture, rather than treating
"simd-json preferred" as a settled performance win without measurement.

### 3. Targeted mutation is a validated win; a fully-typed struct model was correctly ruled out

The benchmark's 4-way comparison confirms targeted mutation (touch only
messages needing coercion) is meaningfully faster and lower-allocation than
a full rebuild *at the same `Value` representation* (~70µs/698 allocations
vs. ~102µs/1319 allocations at 100 messages) — the core design decision
holds up under measurement. A fully-typed-struct baseline is faster still in
raw terms, but the original task explicitly ruled that model out
("do not deserialize into an owned, fully-typed struct-per-field model as
the default path") specifically because of the open-schema requirement
("round-trip unknown fields, do not assume a closed schema") — the
benchmark's naive struct baseline does not attempt that, so it isn't a fully
fair comparison on generality, only on raw throughput for a fixed known
shape.

### 4. Body-size limit: `http_body_util::Limited` directly, not the `DefaultBodyLimit` tower layer

The plan named `DefaultBodyLimit`. That layer is what the `Bytes`/`Json`
*extractors* consult; since the handler takes the raw `Request` itself (to
run the byte-level `coerce_system_messages` rather than deserializing into
an owned struct), it enforces the limit directly with the same underlying
`http_body_util::Limited` mechanism `DefaultBodyLimit` itself is built on.
This is a **deviation in implementation mechanism, not in behavior** — it
lets the 413 case map straight into this crate's own `ProxyError::BodyTooLarge`
and JSON error shape, rather than axum's default plain-text rejection body
(which the requirement explicitly forbids: "NEVER raw upstream error text or
a stack trace" and the confirmed `{"error": {...}}` shape).

### 5. Error-kind vocabulary for `proxy_upstream_errors_total`

`nfr-design/observability-design.md` names exactly `connect_timeout`,
`request_timeout`, `non_2xx` as the `kind` label values. `ProxyError::
UpstreamConnectFailed` (which covers both an actual connect timeout and an
immediate connection-refused) maps to `connect_timeout` per that vocabulary
— there's no separate bucket for "refused" in the confirmed design. A
non-2xx upstream response is *not* a `ProxyError` at all (it's still passed
through verbatim per FR4); its `non_2xx` count is recorded directly in
`server.rs` next to where the upstream status is read.

### 6. Graceful-shutdown test avoids raising real OS signals

`shutdown::signal()` itself waits on `tokio::signal::ctrl_c()`/SIGTERM.
Raising a real signal in an integration test would affect every other test
running in that same process (signal delivery isn't scoped to one thread).
`tests/shutdown.rs` instead drives `axum::serve(...).with_graceful_shutdown(...)`
with a test-controlled `oneshot` channel — the same mechanism `signal()`'s
future provides in production — and asserts the drain behavior that
matters (an in-flight streaming request still completes; the server task
actually exits afterward). `shutdown::signal()`'s own signal-listening code
is therefore not covered by an automated test; see Coverage below.

### 7. Metrics recorder installed once per process via `OnceLock`

`metrics_exporter_prometheus::PrometheusBuilder::install_recorder()` can
only succeed once per process (the `metrics` facade allows one global
recorder). Since the integration-test suites build many `AppState`s within
one test binary process, `metrics::shared_handle()` installs the recorder
lazily on first use and returns a cloned handle on every subsequent call,
falling back to a local (non-global) recorder's handle if installation
ever fails for a reason other than "already installed."

## Test coverage summary

`cargo llvm-cov --all-targets --summary-only`:

| File | Line coverage | Notes |
|---|---|---|
| `transform.rs` | 99.14% | 15 unit + property tests |
| `error.rs` | 100.00% | 7 unit tests, one per variant + a body-shape check |
| `config.rs` | 92.91% | 9 unit tests; the uncovered lines are `ProcessEnv`'s real `std::env::var` wrapper, which the fake-env-backed tests intentionally don't exercise |
| `server.rs` | 89.29% | Covered by 8 integration + 3 logging + 2 metrics + 1 shutdown test; remaining gaps are internal-error branches (malformed outbound request construction) that are hard to trigger without mocking the `http` crate's own builder |
| `metrics.rs` | 76.92% | The `OnceLock` "already installed" fallback branch isn't exercised on every run, depending on test-binary ordering |
| `logging.rs` | 0.00% | `init()` calls `tracing_subscriber::fmt().init()`, which sets the *global* default and can only succeed once per process — deliberately never called from a test (tests instead use `tracing::subscriber::set_default`, a thread-local scope, exercising the same downstream logging behavior without needing to call `init()` itself) |
| `main.rs` | 0.00% | The binary's own `main()`; not exercised by any test (would require spawning the compiled binary as a subprocess) |
| `shutdown.rs` | 0.00% | `signal()`'s real SIGINT/SIGTERM listening code; the *drain mechanism* it feeds is tested (see decision 6 above), but raising real signals in-process was deliberately avoided |
| **TOTAL** | **86.19%** | Exceeds the team's 80% target |

Per team.md's affirmed Testing Posture, 80% is a target, not a hard gate,
and a module falling short is acceptable case-by-case if the suite's
completeness is otherwise satisfactory — the three 0%-covered files above
are each a case of "the only way to exercise this line is to touch global
process state (a global tracing default, a real OS signal, or the compiled
binary's own entry point) that would either be unsafe to do inside a
shared test process or would test the OS/tokio signal-delivery mechanism
itself rather than this crate's code," and each has a same-behavior,
safely-testable alternative that is in fact tested (see decisions 3 and 6).

Test counts by target:

- `cargo test --lib`: 33 tests (`transform`: 17, `config`: 9, `error`: 7).
- `cargo test --test integration`: 8 tests.
- `cargo test --test logging`: 3 tests.
- `cargo test --test shutdown`: 1 test.
- `cargo test --test metrics`: 2 tests.
- **Total: 47 tests, all passing.**

## Zero-copy design — final accounting (as requested in the original task)

A **true, fully zero-copy** implementation was **not** fully achievable
given `serde_json::Value`'s ownership model: `Value` has no borrowed mode,
so parsing a request body into it is one full-owned-tree allocation
regardless of parser (confirmed by the benchmark's allocation counts: 690
allocations for a 100-message fixture even via the leanest path). This is
documented in the `src/transform.rs` module doc, in `README.md`'s
Performance section, and here:

- **What is avoided**: after that one parse, zero additional clones or
  rebuilds — only messages actually needing coercion are mutated, and only
  their `role`/`content` fields specifically (not the whole message object).
  Content prefixing is one right-sized allocation (`String::with_capacity`),
  not an intermediate prefix string plus concatenation.
- **What is not avoidable**: the one initial parse (necessarily allocates
  proportional to input size, since `serde_json::Value` owns every string
  and builds a full object/array tree), and the one final reserialize.
  `preserve_order` is enabled so untouched field *order* round-trips too,
  not just values; non-semantic input formatting (whitespace, non-canonical
  number formatting) is not reproduced by re-serializing a `Value` — that's
  the specific, unavoidable cost of the one full parse under this ownership
  model.
- **Measured cost**: ~690-698 allocations and ~70µs per call at 100
  messages on this development machine (`benches/transform_bench.rs`); see
  README.md for the full comparison table and the caveat about the
  simd-json path's measured (non-)benefit at this workload size.

## Deviations from the plan

1. **simd-json dispatch**: implemented exactly as approved (not simplified
   to serde_json-only), after an initial incorrect assumption was corrected
   mid-dispatch — see decision 2 above. No behavioral deviation; a
   documented performance caveat.
2. **Body-size limit mechanism**: `http_body_util::Limited` directly instead
   of the `DefaultBodyLimit` tower layer — see decision 4 above. No
   behavioral deviation (same 413 outcome, same bound), a deviation in
   *which* axum/tower primitive implements it, made to guarantee the
   confirmed JSON error-body shape on that path.
3. **Two extra library modules beyond the plan's named list**
   (`src/lib.rs`, `src/server.rs`) beyond `main.rs`/`transform.rs`/
   `config.rs`/`error.rs`/`metrics.rs`: a `lib.rs`+thin-`main.rs` split is
   the standard idiomatic pattern for a testable Rust binary (it's what lets
   `tests/`/`benches/` build the router and call `transform` in-process
   without spawning the compiled binary), and `server.rs` holds the
   HTTP-routing/forwarding code the plan's Step 7 describes but doesn't
   assign a specific filename to.
4. **CI workflow added at Step 11**, beyond that step's literal text
   ("lint and format gates"): team.md's affirmed Testing Posture explicitly
   states these checks are "build-enforced... not left to manual discipline
   alone" via a CI workflow, and explicitly distinguishes this from the
   deployment automation that's out of scope. `.github/workflows/ci.yml`
   runs fmt/clippy/test plus a separate `cargo audit` job, no deploy step.
5. **`cargo audit`/`cargo llvm-cov` tooling**: the locally-installed
   `cargo-audit` (0.20.0) failed to parse a CVSS 4.0 entry in the current
   RustSec advisory database; updated to 0.22.2, which resolved it cleanly
   (0 advisories against 250 dependencies). `cargo-llvm-cov` and the
   `llvm-tools-preview` rustup component were not present and were
   installed to produce the coverage table above. Neither is a code change;
   noted here since installing dev tooling wasn't explicitly requested.

## Verification performed

- `cargo test --all-targets`: 47/47 passing.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `cargo audit`: clean (0 advisories, 250 dependencies).
- `cargo llvm-cov --all-targets`: 86.19% line coverage (target: 80%).
- `cargo bench --bench transform_bench`: runs end-to-end; see README.md's
  Performance section for the measured comparison (tracked/reported per
  team practice, not a merge gate).
