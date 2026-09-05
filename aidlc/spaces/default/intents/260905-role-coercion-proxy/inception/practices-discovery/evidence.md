# Evidence — role-coercion-proxy

> **Status: FINAL.** Integrates Step 2 (Lead Draft) evidence, Step 3 support
> review findings from three independent agents, and the Step 4 interview
> decisions. All 9 interview questions were answered, so unresolved
> uncertainty is minimal.

## Project type

**Greenfield.** The repository contains only the AI-DLC scaffold — no
`Cargo.toml`, no `src/`, no build configuration, no prior code to
reverse-engineer. This draft rests on framework defaults (`org.md`) plus the
ideation-phase artifacts, and — as of this integration — the confirmed
human interview, which is the authoritative source wherever it diverges from
either.

## What was inspected

### Lead (Step 2)

1. `aidlc/spaces/default/memory/org.md` — all five sections relevant to this
   stage. `team.md` and `project.md` confirmed empty — no prior affirmed
   practices to re-run against.
2. `.../ideation/intent-capture/intent-statement.md` — problem statement
   (vLLM chat-template crash on mid-conversation system messages), target
   customer, success metrics.
3. `.../ideation/scope-definition/scope-document.md` — full MVP scope:
   single Rust binary, OpenAI-compatible HTTP server, request-transform
   logic, SSE streaming pass-through, zero-copy JSON design constraint,
   `tracing` logging, env-var configuration, graceful shutdown, no-panic
   error handling, explicit quality bar. Deployment/infrastructure/CI-CD
   automation and team formation explicitly out of scope.
4. `.../ideation/scope-definition/intent-backlog.md` — risk-first
   sequencing (transform logic and tests first), Rust toolchain and test
   types.

### Support reviews (Step 3)

- **aidlc-quality-agent** — reviewed testing posture, coverage tooling, CI
  gates, and test-pattern gaps from a QA-lead angle. Recommended: a
  per-module methodology split (tests-first for `transform.rs`, test-after
  elsewhere) rather than a blanket cadence; `cargo-llvm-cov` over
  `cargo-tarpaulin` plus branch coverage, given async Rust's line-coverage
  blind spots and how much correctness lives in error branches; flagged the
  apparent contradiction between "CI execution before merge" and
  "CI/CD automation out of scope" as needing interview resolution; named two
  under-specified MUSTs needing explicit test cases (streaming backpressure,
  `transform.rs` round-trip invariants); recommended clippy deny-lints for
  the no-unwrap/no-panic requirement; flagged the criterion benchmark should
  be tracked, not a hard merge gate; noted `proptest` regression files
  should be committed.
- **aidlc-developer-agent** — reviewed Code Style from a senior Rust
  developer's perspective. Identified that `cargo clippy -D warnings` alone
  does not enable clippy's restriction-group lints (`unwrap_used`,
  `expect_used`, `panic`), so it would not mechanically catch the exact
  failure the scope document forbids; recommended a `[lints.clippy]` table
  or crate-root `#![deny(...)]`. Proposed a typed-error-per-layer convention
  (`TransformError` HTTP-agnostic, a proxy-level error enum owning
  status-code mapping, `anyhow` confined to startup only), aligned with the
  `transform.rs`/`main.rs` module boundary. Named Rust-specific file-layout
  conventions (`#[cfg(test)]` inline for unit/property tests,
  `tests/common/mod.rs` for shared integration-test setup,
  `harness = false` for criterion benches). Flagged HTTP framework choice as
  a Domain Design decision, not this stage's.
- **aidlc-devsecops-agent** — reviewed from a risk-proportionate DevSecOps
  angle: trusted internal pipeline, solo operator, no public internet
  exposure, no infrastructure to secure. Recommended against dedicated SAST/
  DAST/SBOM tooling as disproportionate for this system's shape; recommended
  `cargo audit` as warranted (dependency hygiene against `Cargo.lock`, not
  deployment infrastructure, so unaffected by the deployment-out-of-scope
  carve-out) and `Cargo.lock` committed to the repo; recommended the same
  clippy deny-list as the quality/developer reviewers, plus
  `#![forbid(unsafe_code)]` if the zero-copy transform can stay in safe
  Rust; flagged that light-touch secret scanning (`.env` gitignored, an
  optional `gitleaks` pre-commit hook) is proportionate but not mandatory;
  raised the open question of whether the proxy authenticates outbound to
  `VLLM_BASE_URL` and how; agreed the Deployment section should read "not
  applicable" (this recommendation was **not adopted** — see below); raised
  the CI-execution-vs-out-of-scope tension independently of the quality
  reviewer.

## Interview decisions (Step 4) — including where the human diverged from the drafted default or a reviewer's suggestion

All 9 questions were answered explicitly; every decision below is taken
verbatim from the `[Answer]:` lines in `practices-discovery-questions.md`,
not re-derived.

- **Q1 (Way of Working)** — **Diverges from the drafted org default.** The
  draft carried forward trunk-based development with short-lived feature
  branches and squash-merge. The human rejected this entirely: commits go
  directly to `main` in small logical chunks using Conventional Commits
  message format, with no feature branches and no squash-merge step at all.
- **Q2 (Walking Skeleton)** — Confirms the draft's flagged lean: skip the
  ceremony; risk-first backlog sequencing already serves the purpose.
- **Q3 (Testing methodology)** — **Adopts the quality reviewer's
  suggestion over the draft's blanket default.** The draft's default was
  uniform test-after; the human confirmed the quality reviewer's per-module
  split instead: tests-first for `transform.rs` specifically, test-after
  everywhere else. Recorded as `Methodology: custom` per the stage's own
  rule for mixed cadences.
- **Q4 (Coverage target/tooling)** — **A hybrid, not exactly either offered
  option.** Adopts the quality reviewer's tooling recommendation
  (`cargo-llvm-cov`, line + branch coverage) and the draft's 80% figure, but
  the human added a qualifier neither the draft nor the reviewer had
  proposed: 80% is a target, not a hard gate — a shortfall is acceptable
  case-by-case if the test suite's completeness is otherwise satisfactory.
- **Q5 (CI scope)** — Confirms both reviewers' framing: "deployment
  automation out of scope" does not exclude a test/lint-only CI workflow
  (`cargo test`/`clippy`/`fmt` on push, no deploy step); that workflow is
  in-scope project tooling.
- **Q6 (Error typing + lints)** — Confirms both the developer reviewer's
  typed-error-per-layer convention and the clippy deny-lint recommendation
  (independently raised by the quality, developer, and devsecops reviewers)
  in full — "Yes to both."
- **Q7 (Deployment section)** — **Diverges from the drafted default AND
  from the devsecops reviewer's explicit recommendation.** Both the lead's
  Step 2 draft and the devsecops reviewer recommended marking this section
  "not applicable — deployment is out of scope." The human chose the
  opposite: keep general deployment guidance in the section anyway, in case
  it's useful later, even though deployment work itself remains out of
  scope for this workflow's deliverable.
- **Q8 (Dependency scanning)** — Confirms `cargo audit` as a required
  check, as recommended by the devsecops reviewer; declines the fuller
  `cargo-deny`.
- **Q9 (Outbound auth to vLLM)** — Resolves the devsecops reviewer's
  flagged open gap: the proxy passes the inbound request's existing
  `Authorization` header through unchanged; no separately configured static
  upstream credential.

## Unresolved uncertainty

None of substance remain — all 9 interview questions were answered and the
Consolidated Summary Confirmation was accepted as "Looks correct" with no
requested changes. Two implementation-level details are deliberately left
open for a later stage rather than this one, per the developer reviewer's
own note: the HTTP framework choice (axum / actix-web / warp+hyper) and the
full module decomposition beyond the `transform.rs`/`main.rs` boundary are
Domain Design decisions, not practices-discovery decisions, since they
depend on responsibilities not yet designed.
