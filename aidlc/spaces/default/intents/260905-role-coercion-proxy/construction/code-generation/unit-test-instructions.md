# Unit Test Instructions — qwaude-proxy (role-coercion-proxy)

## Test Framework Setup

- **Unit + property tests**: Rust's built-in `#[test]` harness plus `proptest` (dev-dependency), inline as `#[cfg(test)] mod tests` within `src/transform.rs` (per `team-practices.md`'s affirmed module layout).
- **Integration tests**: `tests/` directory at the workspace root, using `wiremock` as the mock upstream (dev-dependency), with shared setup factored into `tests/common/mod.rs`.
- **Benchmark**: `criterion` (dev-dependency), `benches/transform_bench.rs`, `[[bench]] name = "transform_bench" harness = false` in `Cargo.toml`.

## Exact Unit-Scoped Commands

Every command below is scoped to this project's own test/bench targets — there is only one unit (zero-Unit, stage-level project), so these ARE the project's full commands, but each is still name-scoped rather than a bare catch-all:

- Runner readiness (Step 2, before any test-first work): `cargo test --lib --no-run` — confirms the crate compiles and the test harness is wired, before `transform.rs` exists.
- `transform.rs` unit + property tests (Step 3/4): `cargo test --lib transform::tests`
- `config.rs` unit tests (Step 5): `cargo test --lib config::tests`
- `error.rs` unit tests (Step 6): `cargo test --lib error::tests`
- Integration tests (Step 7): `cargo test --test integration`
- Logging test (Step 8): `cargo test --test integration logging` (or its own `tests/logging.rs` target — `cargo test --test logging`)
- Graceful-shutdown test (Step 9): `cargo test --test integration shutdown` (or its own `tests/shutdown.rs` target — `cargo test --test shutdown`)
- Metrics test (Step 10): `cargo test --test integration metrics` (or its own `tests/metrics.rs` target — `cargo test --test metrics`)
- Lint/format (Step 11): `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
- Benchmark (Step 12): `cargo bench --bench transform_bench`
- Full suite (final verification, also used by Build and Test): `cargo test --all-targets`

## Expected Coverage Targets

80% line + branch coverage (via `cargo-llvm-cov`) as a target, not a hard gate — acceptable to fall short case-by-case if the test suite's completeness (happy path + edge cases + invariants) is otherwise satisfactory, per `team-practices.md`'s affirmed Testing Posture.

## Test Volume (Standard strategy)

5-8 tests per component, per the Testing Contract's `strategy_volume` obligation:
- `transform.rs`: ~7 unit tests (leading system message; single mid-stream coercion; multiple mid-stream coercions; string content; content-block-array content; missing/non-standard content shape; unknown-field round-trip) + property tests for the 3 invariants (length preserved, no non-leading system role, non-rewritten fields preserved).
- `config.rs`: ~4-5 tests (one per env var's default + override behavior).
- `error.rs`: ~4-5 tests (one per `ProxyError` variant's status code + body shape).
- Integration (`main.rs`, via `wiremock`): ~7-8 tests (non-streaming round-trip; streaming pass-through with early-byte assertion; large message array; malformed JSON → 400; upstream connection refused → 502; upstream timeout → 504; oversized body → 413; `Authorization` header forwarded unchanged).
- Logging, shutdown, metrics: 1-2 targeted tests each (narrower components, per the Testing Contract's "omitting genuinely inapplicable layers without changing the methodology" allowance).

## Mocking/Stubbing Guidance

- The upstream vLLM server is mocked with `wiremock` for all integration tests — no real vLLM instance required to run the suite.
- `transform.rs`'s unit and property tests require no mocking — it is a pure function with no I/O.
- No database or external state store exists to mock (stateless design, Domain Design ADR-002).

## Test Data Management

- `transform.rs` test inputs are inline JSON literals/fixtures within the test module (or a small `tests/fixtures/` directory of sample request bodies for the larger integration-test payloads), covering the shapes named in Step 3.
- `proptest` generates arbitrary valid `messages` arrays via a custom `Strategy` respecting the OpenAI chat-completion message shape (role, content as string or content-block array, optional fields).
- Any `proptest`-discovered failing case is committed to `proptest-regressions/transform.txt` (not gitignored), per `team-practices.md`.
