# Team Practices — role-coercion-proxy

> **Status: FINAL.** Integrated at Step 5 from the Step 2 lead draft, the
> three independent support reviews (quality, developer, devsecops), and the
> completed 9-question human interview. This is the version promoted to
> `memory/team.md` on approval.

## Way of Working

We commit **directly to `main`**, in small logical chunks, using
**Conventional Commits** message format (e.g. `feat(api): ...`,
`tests(transform): ...`, `fix(transform): ...`). There are no feature
branches and no squash-merge step for this project — this replaces the
org-default trunk-based/short-lived-branch/squash-merge practice for
`role-coercion-proxy` specifically (Q1).

Each commit should represent one coherent, reviewable unit of change (one
function's implementation plus its tests, one config addition, etc.) rather
than a batched end-of-session dump — the intent of "small logical chunks" is
to keep `main`'s history readable and bisectable without relying on branch
structure to do it.

## Walking Skeleton

We **skip the walking-skeleton ceremony** for this project (Q2). The
backlog's risk-first sequencing — build and test the message-transform logic
(`transform.rs`, the highest-risk, most isolated part and the actual cause of
the original crash) before HTTP wiring — already serves the walking
skeleton's purpose of proving the riskiest piece first. Bolt 1 is the
transform-logic work itself, not a separate thin end-to-end slice.

## Testing Posture

- **Methodology**: custom
- **Ordering**: write property-based tests before or alongside the
  implementation for `transform.rs` specifically (tests-first for that
  module only), and for every other layer — HTTP wiring, configuration,
  logging, shutdown, error handling — implement the layer first and then
  write and run that layer's tests (test-after), per the confirmed interview
  answer (Q3).

`custom` is used here per this stage's own rule for a mixed-cadence answer:
`transform.rs` is tests-first, everything else is test-after — a single
uniform label would misrepresent the confirmed decision.

Coverage, tooling, test types, and CI scope (Q4, Q5):

- **Coverage target/tooling**: aim for 80% coverage (line + branch, measured
  via `cargo-llvm-cov`), but this is a **target, not a hard gate** — a module
  falling short of 80% is acceptable case-by-case if the test suite's
  completeness (happy path + edge cases + invariants covered) is otherwise
  satisfactory. `cargo-llvm-cov` is used in preference to `cargo-tarpaulin`
  for better accuracy against this project's async (`tokio`-driven) code.
- **Test types**: unit tests, property-based tests (proptest/quickcheck) for
  `transform.rs`, integration tests against a mock upstream capable of
  streaming chunked responses (not just a canned body), and a criterion
  benchmark comparing the zero-copy transform against a naive
  full-deserialize baseline.
- **Benchmark is tracked/reported, not a hard merge gate.** `cargo test`,
  `cargo clippy -D warnings`, and `cargo fmt --check` passing clean is the
  merge bar; the criterion benchmark result is not required to show zero
  regression to merge, since bench numbers are inherently noisy across
  machines.
- **CI scope**: "deployment/infrastructure/CI-CD automation is out of scope"
  refers to deployment/hosting automation only. A CI workflow that runs
  `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, and
  `cargo audit` on every push/PR — with no deploy step — is in-scope project
  tooling, not deployment infrastructure (Q5). These checks are therefore
  build-enforced by that workflow, not left to manual discipline alone.
- **Streaming backpressure** ("streams SSE responses through without full
  buffering, preserving backpressure") is a specific, testable claim and
  needs an explicit test case (a slow consumer against a fast/large upstream
  stream, asserting the proxy never buffers the full body) rather than being
  left implicit inside a generic "integration tests" bucket.
- **`transform.rs` invariants to name explicitly** as property tests:
  `messages[0]` with `role: "system"` is never mutated; every other `system`
  message becomes `role: "user"` with the notice prefix prepended; every
  other field (`tool_calls`, `tool_call_id`, `name`, cache-control hints,
  unknown fields) round-trips byte-identical; both string-content and
  content-block-array content shapes are handled with equivalent effect.
- `proptest`'s failing-case seed files (`proptest-regressions/<module>.txt`)
  are committed to the repo, not gitignored, so a once-found counterexample
  can never silently regress.

## Deployment

Deployment/infrastructure work itself remains **out of scope** for this
workflow's deliverable (the binary — code, tests, benchmarks, README — not a
deployed/running service). Per the confirmed interview answer, we
**keep general deployment guidance** in this section anyway for future use,
rather than marking it "not applicable" (Q7 — this is the opposite of the
lead's own Step 2 draft and the devsecops reviewer's recommendation, both of
which suggested "not applicable"; the human overrode both):

- `org.md`'s default posture — deploy on merge to staging, with a separate
  manual approval (tech lead + product owner sign-off) gating production —
  is the guidance carried forward here for whenever a future workflow adds
  deployment to this proxy.
- Marking this "not applicable" would have removed only the *deploy-time*
  guidance; it never covered the *build-time* gates (clippy deny-lints,
  `cargo fmt --check`, `cargo audit`, the test suite), which live under
  Testing Posture / Code Style above and apply regardless of this section.
- Outbound authentication to `VLLM_BASE_URL`: the proxy **passes the
  inbound request's existing `Authorization` header through unchanged** —
  there is no separately configured static credential for the upstream
  connection (Q9). Whatever credential the caller presents is forwarded
  as-is; the proxy does not mint, store, or log it.

## Code Style

We defer to project-level configuration, specialized for Rust:

- **Formatter**: `cargo fmt` (rustfmt), configured via `rustfmt.toml` at the
  repo root if non-default settings are needed. `cargo fmt --check` runs
  before merge; failure blocks the change.
- **Linter**: `cargo clippy -D warnings` (default lint groups escalated to
  errors) **plus** an explicit `[lints.clippy]` table in `Cargo.toml` (or
  equivalent crate-root `#![deny(...)]`) enabling `unwrap_used`,
  `expect_used`, and `panic` — `-D warnings` alone does not catch these,
  since they live in clippy's restriction group, which is off by default.
  This mechanically enforces the "no `unwrap()`/`panic!()` on the request
  path" requirement rather than relying on review alone (Q6). Scope the
  deny-list at minimum to the request-handling modules (`main.rs`'s handler,
  `transform.rs`), with a narrow `#[allow(...)]` in `#[cfg(test)]` modules
  where `.unwrap()` is normal test code.
- **Dependency scanning**: `cargo audit` (RUSTSEC advisory scanning against
  `Cargo.lock`) is a required check before merge, run either locally or as
  part of the test/lint CI workflow described under Testing Posture (Q8).
  `Cargo.lock` is committed to the repo (not gitignored) — this is what
  makes `cargo audit` and reproducible builds possible.
- **Error-handling convention**: a typed error type per layer, matching the
  `transform.rs`/`main.rs` boundary (Q6):
  - `TransformError` (or similar), owned by `transform.rs`, describing only
    transform-time failures — stays free of HTTP/transport concerns (no
    status codes, no response types), preserving `transform.rs`'s
    independence from HTTP wiring.
  - A request/proxy-level error enum (e.g. `ProxyError`, in `main.rs` or a
    small `error.rs`) that composes `TransformError` plus
    upstream-forwarding failures, and is the only place that maps errors to
    HTTP status codes and the JSON error-body shape.
  - `anyhow`, if used at all, is confined to `main()`'s own
    startup/config-parsing path — never the request path.
- **Naming conventions**: Rust-idiomatic — `snake_case` for functions,
  variables, and modules; `CamelCase` for types and traits;
  `SCREAMING_SNAKE_CASE` for constants. No project-wide rename rules beyond
  the language default.
- **Module layout**: `transform.rs` isolated from `main.rs` — the
  request-transform logic (highest design risk) is kept independently
  unit/property-testable, separate from HTTP-server wiring. Unit and
  property tests for `transform.rs` live inline as `#[cfg(test)] mod tests`
  in the same file. Integration tests against the mock upstream live under
  the top-level `tests/` directory, with shared mock-upstream setup factored
  into `tests/common/mod.rs`. `benches/` entries need
  `[[bench]] name = "..." harness = false` in `Cargo.toml` for criterion to
  run them.
