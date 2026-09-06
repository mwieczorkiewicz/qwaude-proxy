# Test Results — qwaude-proxy — Build and Test

## Build status

**Success.** `cargo build --all-targets` and `cargo build --release` both complete cleanly (release build: `opt-level = 3`, `lto = true`), with no warnings.

## Test results

**49 / 49 passing** across 7 test suites (0 failed, 0 skipped):

| Suite | Tests | Result |
|---|---|---|
| `cargo test --lib` (`transform`, `config`, `error` inline `#[cfg(test)]` modules) | 33 | Pass |
| `tests/integration.rs` | 8 | Pass |
| `tests/logging.rs` | 3 | Pass |
| `tests/shutdown.rs` | 1 | Pass |
| `tests/metrics.rs` | 2 | Pass |
| `tests/build_and_test_checks.rs` (added this stage) | 2 | Pass |
| **Total** | **49** | **Pass** |

No failure details to report — every test passed on the first run at this stage, including the two tests added to close coverage gaps (see Build and Test Summary).

## Coverage report

`cargo llvm-cov --all-targets --summary-only`: **86.19% line coverage**, exceeding the team's 80% target (a target, not a hard gate, per `team.md`'s affirmed Testing Posture). Full per-file breakdown and the rationale for the three 0%-covered files (each a case of "the only way to exercise this line touches global process state") is in `code-generation/code-summary.md` and is unchanged by this stage's additions — the two new tests in `tests/build_and_test_checks.rs` add coverage to already-instrumented paths (`src/server.rs`'s health handler and request-handling logic) rather than opening new uncovered files.

## Static checks

| Check | Result |
|---|---|
| `cargo clippy --all-targets -- -D warnings` | Clean — no issues found |
| `cargo fmt --check` | Clean |
| `cargo audit` | Clean — 0 advisories found against 250 dependencies (RustSec advisory database, 1239 total advisories, last updated 2026-09-02) |

## Performance evidence

`cargo bench --bench transform_bench -- --test` re-run cleanly at this stage (post-rename), confirming the benchmark still compiles and executes across all three fixture sizes (10/100/1000 messages) and all four comparison paths. Full statistical numbers (allocation counts) are in `performance-test-instructions.md` and match the code-generation stage's original measurements — no regression introduced by the rename or by this stage's additions.

## Additions made at this stage

Two tests were added to close coverage gaps the Step 1 target inventory found — claimed by design (NFR5.2, NFR1.4) but with no dedicated automated check in the code-generation output:

1. `health_endpoint_returns_200` — `GET /health` returns 200, verified independent of upstream (vLLM) reachability.
2. `concurrent_requests_are_handled_without_serialization` — 20 concurrent requests against a mocked upstream with an artificial delay complete in well under half the fully-serial time bound.

Both are in-stage additions (test code, not application code changes) — no code-generation loop-back was needed; this is ordinary Build and Test scope (Step 9 explicitly allows executable target checks within this stage's own remit).

## Target Verification Matrix

See `build-and-test-summary.md` for the full matrix; every applicable target resolves to `Met`.

## Loop-Back Log

Not applicable — no build or test failure occurred at this stage, so no loop-back to Code Generation was needed.
