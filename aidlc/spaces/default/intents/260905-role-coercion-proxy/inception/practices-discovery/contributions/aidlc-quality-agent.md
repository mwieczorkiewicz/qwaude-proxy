**Collaborator:** aidlc-quality-agent

## Contribution

Reviewed `team-practices.md`, `discovered-rules.md`, and `evidence.md` against the scope document and intent backlog, from a QA-lead angle: testing posture, coverage tooling, CI quality gates, test/code patterns, and interview gaps.

### Testing Posture — methodology: test-after-for-everything under-serves `transform.rs`

The draft's blanket `test-after` methodology (carried from `org.md`'s default) is reasonable for the HTTP-wiring, configuration, logging, shutdown, and error-handling layers — that code is mostly mechanical glue where writing tests after a first working pass is efficient and low-risk.

`transform.rs` is a different risk profile, and the draft's own evidence says so: it is called out as "the highest design risk" (serde's ownership model), it is pure and I/O-free, and it is explicitly isolated to be unit/property-testable on its own. That combination — pure function, highest design risk, root cause of the production bug being fixed — is exactly the profile where writing the properties first earns its keep: defining the correctness invariants (see below) before or alongside the implementation forces the edge cases that caused the original vLLM crash to be named up front, rather than discovered only if someone happens to write a test for them afterward. Recommend the interview offer a **per-layer methodology** for this module specifically — property-test-first / TDD for `transform.rs`, test-after for the rest — which the stage protocol itself anticipates: its own rule says to record `Methodology: custom` "whenever the answer mixes cadences," so this is a supported outcome, not a deviation the lead needs to invent room for. The interview should ask this as an explicit either/or rather than let the org default silently cover the one module where it fits worst.

### Coverage floor — 80% blanket line coverage needs sharpening, not just confirming

The draft correctly flags that the `feature` scope tag driving the 80%-floor default isn't independently re-confirmed for this workflow, and that's worth resolving. But even once the scope tag is confirmed, a single blanket 80% **line** floor is a poor fit for this codebase's shape:

- `transform.rs` is pure and fully unit/property-testable — it should realistically clear 90%+ and the floor should not be the ceiling the team aims for on the module that matters most.
- HTTP wiring, SIGINT/SIGTERM graceful shutdown, and SSE streaming are notoriously hard to line-cover with unit tests in async Rust; they're properly covered by the integration-test layer, not unit tests, and coverage tooling (`cargo-tarpaulin` in particular) has known blind spots around `tokio`-driven async code and macro-expanded code. Recommend `cargo-llvm-cov` over `cargo-tarpaulin` for this project specifically, for better async accuracy, and recommend **branch coverage** alongside line coverage given how much of this system's correctness lives in error-handling branches (malformed input, upstream failure → 400/502/504).

Flagging this as an interview point: confirm the coverage tool, confirm whether the floor is measured per-module or project-wide, and confirm branch coverage is tracked given the "no panics/unwraps on the request path" requirement lives almost entirely in branch logic.

### Apparent scope contradiction: "CI execution before merge" vs. "CI/CD automation is out of scope"

`org.md`'s feature-scope testing-posture floor (which the draft carries forward, pending confirmation) adds "CI execution before merge" as a requirement. The scope document twice states deployment/infrastructure/CI-CD automation is out of scope for this workflow. These aren't obviously the same claim, but they read close enough that the interview should resolve the ambiguity explicitly: does "CI execution before merge" mean an actual pipeline (e.g., a GitHub Actions workflow file) gets built as part of this deliverable — which would contradict the out-of-scope declaration — or does it mean "`cargo test && cargo clippy -D warnings && cargo fmt --check` are run manually/locally before every merge, with no pipeline authored"? The draft doesn't surface this tension; it should be one of the interview's Testing Posture questions.

### Gaps: stated MUSTs in the scope document without a corresponding test obligation

Two design constraints in the scope document are stated as MUSTs but aren't yet named as specific test cases anywhere in the draft:

1. **Streaming backpressure** — "streams SSE responses through without full buffering... preserving backpressure" is a specific, testable claim, not just "integration tests" in general. It needs an explicit test case: a slow consumer against a fast/large upstream stream, asserting memory does not grow with response size (or asserting the proxy never accumulates the full body before forwarding). Generic "integration tests against a mock upstream" as currently drafted could pass without ever exercising this.
2. **Zero-copy/round-trip correctness invariants** for `transform.rs` — the property tests should be named, not left implicit: (a) `messages[0]` with `role: "system"` is never mutated; (b) every other `system` message becomes `role: "user"` with the notice prefix prepended; (c) every field other than `role`/content-prefix (`tool_calls`, `tool_call_id`, `name`, cache-control hints, unknown fields) round-trips byte-identical; (d) string-content and content-block-array content shapes are both handled with equivalent effect. Naming these now gives Code Generation/Build-and-Test a concrete checklist instead of re-deriving it from the scope prose later.

### Missing quality-gate that directly operationalizes a stated MUST

"No `unwrap()`/`panic!()` on the request path" is currently framed only as something tests should catch. Rust has a sharper tool for exactly this: recommend `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` (scoped to the request-path modules, or project-wide with narrow `#[allow]`s in test code) as an additional Code Style / quality-gate bullet. This turns a design intention into a compiler-enforced gate rather than something that only fails if someone happens to write the right test — strictly stronger than test coverage alone for this specific requirement, and cheap to add.

### Benchmark: shouldn't silently become a hard merge gate

The draft lists the criterion benchmark as an in-scope deliverable (correct — it's explicit in the scope doc) but doesn't say whether it's a blocking CI gate. Benchmark numbers are inherently noisy across machines/CI runners; recommend the interview clarify that the criterion benchmark is **tracked/reported**, not a hard pass/fail merge gate, unless the team explicitly wants a regression-detection setup (e.g., `criterion`'s built-in baseline comparison, or `critcmp`) with an agreed tolerance. Otherwise "clippy/fmt/tests clean" quietly risks being read as "and the benchmark must show zero regression," which is a much higher bar than the scope document actually states.

### Smaller test-pattern notes

- `proptest` persists failing-case seeds to `proptest-regressions/<module>.txt` by default on shrink. Recommend explicitly stating these files are committed to the repo (not gitignored) so a once-found counterexample can never silently regress — a common miss.
- The mock-upstream tooling choice for integration tests isn't named. Since streaming SSE forwarding is a first-class requirement, the mock needs to be able to itself stream chunked responses convincingly (not just return a canned body) — worth naming `wiremock` or a small in-process `axum`/`hyper` test server as the candidate rather than leaving "mock upstream" unspecified until Build-and-Test.
- `org.md`'s Testing Posture also carries an independent **Test Strategy** axis (Minimal/Standard/Comprehensive, set via `--test-strategy`, governing test volume per component) that the draft's coverage/tooling notes never reference. This is a separate knob from the coverage-floor percentage and determines how many tests per component Code Generation should actually produce — worth confirming or at least cross-referencing at the interview so volume expectations are set, not just the floor.

## Positions

- AGREE: The draft's coverage/tooling/scope bullets (test types, `cargo clippy -D warnings`, `cargo fmt --check`) are lifted verbatim from the scope document and intent backlog rather than invented — high-fidelity capture of what's actually stated. — Matches the actual project artifacts; nothing here needed correction.
- AGREE: Risk-first sequencing (transform logic and its tests first, in isolation, before HTTP wiring) is sound test-pyramid practice, not just a scheduling choice — it puts the highest-risk, purely-logic code under the heaviest test scrutiny earliest. — Reduces the chance the original crash class resurfaces, and matches standard QA risk-based test prioritization.
- AGREE: Flagging the testing methodology, the coverage-floor scope tag, and the deployment section as open interview items rather than silently assuming an answer. — Correctly distinguishes framework defaults from confirmed team intent, per this stage's own evidentiary discipline.
- OBJECT: A single blanket `test-after` methodology for the whole project is too coarse for `transform.rs` specifically — recommend the interview offer a per-module split (property-test-first for `transform.rs`, test-after elsewhere), recorded as `Methodology: custom` per the stage's own rule for mixed cadences. — The highest-risk, most-testable module is exactly where test-after gives up the most risk-reduction value versus test-first.
- OBJECT: The 80% coverage floor should be sharpened (tool choice, line vs. branch, per-module vs. project-wide) rather than confirmed as a flat number, and the "CI execution before merge" wording needs to be reconciled with the explicit CI/CD-automation-is-out-of-scope declaration. — As currently drafted, both are confirmable-as-written but would leave real ambiguity about what "clean" and "before merge" concretely require.
