# Practices Discovery — Interview Questions

This project is greenfield, so these questions offer the organization's framework defaults as suggested starting points — not established team facts — for you to confirm or change. Three independent reviewers (quality, developer, devsecops) also looked at the draft and raised a few specific points, folded into the questions below.

## Q1. How should changes get merged into your main branch while you build this?

The organization's default is trunk-based development: short-lived feature branches merged straight into `main`, with each unit of work squash-merged into one clean commit (all the commits on a branch collapsed into a single commit on `main`).

A. Yes, that's fine — trunk-based, short-lived branches, squash-merge into `main`
B. Something else — I'll specify
C. Not yet defined
X. Other (please specify)

[Answer]: X. Other — commit directly to main in small logical chunks, using Conventional Commits style messages (e.g. `feat(api): ...`, `tests(transform): ...`). No feature branches, no squash-merge step.

## Q2. Build a thin end-to-end slice first? A walking skeleton is a minimal version that runs the whole way through, built first to prove the pieces connect before the real features go in.

This is a single-unit build already sequenced risk-first — the message-transform logic (the highest-risk, most isolated part) gets built and tested before the HTTP wiring. A separate walking-skeleton step might duplicate that.

A. No — skip it; the risk-first backlog sequencing (transform logic proven first) already serves that purpose
B. Yes — build a minimal end-to-end HTTP round-trip (with a no-op transform) before implementing the real coercion logic
C. Not yet defined
X. Other (please specify)

[Answer]: A. No — skip it

## Q3. How should tests be written relative to the code they test?

The quality reviewer suggested writing tests before the code specifically for the message-transform logic (`transform.rs`) — since that's the highest-risk, most isolated part of this fix, and the module that actually caused the original crash — while writing tests after the code everywhere else (HTTP wiring, config, logging, shutdown).

A. Yes — tests-first for `transform.rs` specifically (property-based tests written alongside/before the transform logic), tests-after for everything else
B. Tests-after everywhere, uniformly — no special treatment for `transform.rs`
C. Tests-first everywhere
D. Not yet defined
X. Other (please specify)

[Answer]: A. Tests-first for transform.rs only

## Q4. What test-coverage target and tooling should this project track?

This project's scope doesn't map to one of the framework's standard coverage-floor presets, so no floor is already fixed. The quality reviewer recommended `cargo-llvm-cov` over `cargo-tarpaulin` (more accurate for async Rust code) and tracking branch coverage in addition to line coverage, since most of this proxy's correctness lives in error-handling branches.

A. 80% line coverage using `cargo-llvm-cov`, also tracking branch coverage
B. A different coverage target/tool — I'll specify
C. No specific coverage floor — rely on the test suite's completeness rather than a numeric target
X. Other (please specify)

[Answer]: X. Other — aim for 80% (line + branch, via cargo-llvm-cov) as a target, but accept less than 80% on a case-by-case basis if the test suite's completeness (happy path + edge cases + invariants covered) is otherwise satisfactory. Not a hard gate.

## Q5. Does "no CI/CD automation in scope" rule out a basic test/lint check that runs automatically on every change?

Earlier, deployment/infrastructure/CI-CD automation was recorded as out of scope. But the workflow plan already includes a later stage that wires `cargo test`, `cargo clippy`, and `cargo fmt --check` into an automated check — not a deployment pipeline, just enforcement that the tests and lint gates pass on every change (e.g. a GitHub Actions workflow file).

A. Yes — "out of scope" meant deployment/hosting automation only; a workflow that just runs tests/clippy/fmt on every push or PR is fine and expected
B. No — testing and linting stay a manual, locally-run discipline; no automated check-on-push file at all
C. Not yet defined
X. Other (please specify)

[Answer]: A. Deployment automation only is excluded — a test/lint CI workflow is fine

## Q6. Should each layer define its own error type, and should the linter specifically flag unwrap/expect/panic calls?

The developer reviewer proposed: a typed error type for the transform logic, kept separate from a typed error type for the HTTP/request layer (which owns translating errors into the right status code and JSON error body) — matching the project's own `transform.rs`/`main.rs` boundary. They also noted that `cargo clippy -D warnings` alone does NOT catch `unwrap()`/`expect()`/`panic!()` calls — those need to be explicitly enabled as clippy lints to mechanically enforce "no panics on the request path," rather than relying on code review alone.

A. Yes to both — a typed error type per layer, and explicit clippy lints (`unwrap_used`, `expect_used`, `panic`) enabled so the build itself catches a stray panic/unwrap
B. Different error-handling or lint approach — I'll specify
C. Not yet defined
X. Other (please specify)

[Answer]: A. Yes to both

## Q7. Since deployment/infrastructure work is out of scope, should the practices document just say "not applicable" for deployment, instead of carrying the organization's general deploy-on-merge/staging default?

A. Yes — mark deployment "not applicable" for this project
B. No — keep some deployment guidance anyway, in case it's useful later
C. Not yet defined
X. Other (please specify)

[Answer]: B. Keep some deployment guidance anyway

## Q8. Should dependency vulnerabilities be checked automatically?

`cargo audit` checks your dependencies against a known-vulnerability database. `cargo-deny` does that plus license and source-pinning policy checks — more thorough, more setup.

A. Yes, `cargo audit` as a required check — lightweight and sufficient for a project this size
B. Yes, the fuller `cargo-deny` (advisories + license + source policy)
C. No automated dependency scanning
D. Not yet defined
X. Other (please specify)

[Answer]: A. cargo audit required

## Q9. Does the proxy need to send any credential to vLLM when forwarding requests, and if so, how should that be configured?

A. No credential needed — vLLM is reachable without authentication on this internal network
B. Yes — a separate environment variable holds a static credential for the outbound connection
C. Yes — pass through the inbound request's existing auth header unchanged
D. Not yet defined
X. Other (please specify)

[Answer]: C. Pass through the inbound request's existing auth header unchanged

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
