# Performance Test Instructions — qwaude-proxy

Generated beyond the Standard strategy's default (which names only integration tests) because real NFR performance requirements exist (`nfr-requirements/performance-requirements.md`, `nfr-design/performance-design.md`) and the original task explicitly required a benchmark comparing the zero-copy transform against a naive baseline.

## Test framework setup

`criterion` (dev-dependency, `harness = false` in `Cargo.toml`) drives `benches/transform_bench.rs`, a 4-way allocation/latency comparison at 10/100/1000-message array sizes:
1. Zero-copy targeted mutation (production path, simd-json dispatch)
2. Targeted mutation, serde_json parse only (isolates the simd-json contribution)
3. Naive full rebuild via `Value` (same representation, no targeted-mutation optimization)
4. Naive full rebuild via typed structs (the model the original task explicitly ruled out, kept as a raw-throughput reference point)

## How to run

```bash
# Full statistical run (multiple seconds per benchmark group; produces HTML reports under target/criterion/)
cargo bench --bench transform_bench

# Quick smoke-test (single iteration per benchmark, confirms nothing broke, no full statistical sampling)
cargo bench --bench transform_bench -- --test
```

Both commands are scoped to the single `transform_bench` target — never a bare `cargo bench`.

## Expected coverage / targets

Per `team.md`'s affirmed Testing Posture: **the benchmark is tracked and reported, not a hard merge gate** — `cargo test`, `cargo clippy -D warnings`, and `cargo fmt --check` passing is the actual merge bar. NFR1.1/1.2/1.3's numeric targets (see `nfr-requirements/performance-requirements.md`) are therefore verified by evidence (the benchmark runs and reports real comparative numbers), not by a pass/fail threshold enforced in CI.

Allocation counts (single call, 100-message fixture, ~1/5 mid-stream system), as measured at this stage:

| Path | Allocations |
|---|---|
| Zero-copy targeted mutation (prod, simd-json dispatch) | 698 |
| Targeted mutation, serde_json parse only | 690 |
| Naive full rebuild via `Value` (same representation) | 1319 |
| Naive full rebuild via typed structs | 480 |

The targeted-mutation strategy (698/690 allocations) is a clear, measured win over the naive full-`Value`-rebuild baseline (1319 allocations) at the same representation — this is the comparison NFR1.2/1.3 actually care about, since the typed-struct baseline was explicitly ruled out by the original task's open-schema requirement and isn't a fair comparison on generality.

## Load / concurrency testing

A dedicated concurrency integration test (`tests/build_and_test_checks.rs::concurrent_requests_are_handled_without_serialization`, added at this stage — see Build and Test Summary) covers NFR1.4 directly: 20 concurrent requests against a mocked upstream with an artificial per-request delay, asserting total wall-clock time is well under the fully-serial bound. This is run via `cargo test`, not `cargo bench` — a full load-testing tool (k6, Locust) was judged disproportionate to this project's confirmed scale (modest internal load, tens of concurrent requests, no fixed RPS target).

## Regression detection

No automated benchmark-regression gate exists (consistent with "tracked/reported, not a hard merge gate"). If a future change is suspected of regressing performance, re-run `cargo bench --bench transform_bench` and compare against the numbers in this file and `README.md`'s Performance section.
