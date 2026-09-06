# Quality Gates — qwaude-proxy

## Sources

- [Q1], [Q3] — `ci-pipeline-questions.md`
- [memory:M-team-testing-posture] — `team.md` § Testing Posture: coverage is a *target*, not a hard gate; the criterion benchmark is *tracked*, not a hard merge gate; `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check` passing clean **is** the merge bar
- [memory:M-project-mandated] — `project.md` § Mandated: `unwrap_used`/`expect_used`/`panic` clippy deny-lints always enabled; `cargo audit` always run before merge
- `build-and-test/test-results.md` — this stage's `docker` gates supersede nothing there; they add a fourth, independent build-enforced check

## Hard gates (block merge / block publish)

Enforced automatically by CI — a failure here fails the workflow run and blocks the change, per `team.md`'s "build-enforced, not left to manual discipline alone":

| Gate | Job | Command | Failure mode |
|---|---|---|---|
| Formatting | `test` | `cargo fmt --check` | Non-zero exit on any unformatted file |
| Lint (incl. no `unwrap`/`expect`/`panic`) | `test` | `cargo clippy --all-targets -- -D warnings` | Any clippy warning fails the build; the crate's own `[lints.clippy]` table additionally denies `unwrap_used`/`expect_used`/`panic` at the source level, so a stray one fails `cargo build` itself, before CI even runs |
| Test suite | `test` | `cargo test --all-targets` | Any failing test fails the job |
| Dependency vulnerabilities | `audit` | `rustsec/audit-check@v2` | Any RUSTSEC advisory against `Cargo.lock` fails the job |
| Container compiles | `docker` | `docker build` (via `docker/build-push-action`, `push:false, load:true`) | Any compile failure inside the Alpine/musl builder stage fails the job — this is a *second*, independently-toolchained compilation of the same crate (musl vs. the `test` job's default glibc target), catching target-specific breakage the `test` job alone would miss |
| Container serves traffic | `docker` | smoke-test script: poll `GET /health` (≤15s), then assert `GET /health` and `GET /metrics` both return `200` | Timeout or non-200 fails the job and blocks publish — added this stage as the "e2e coverage" the human asked for [Q3] |

The `docker` job's `needs: [test, audit]` means a `cargo fmt`/`clippy`/`test`/`audit` failure never even reaches the container build — no wasted CI minutes building an image from code already known to fail.

## Publish gate (additional, GHCR-only)

Publishing to `ghcr.io/mwieczorkiewicz/qwaude-proxy` requires **all** of the above hard gates to pass, **plus**:

| Condition | Enforced by |
|---|---|
| Event is `push` (not `pull_request`) | `if: github.event_name == 'push'` on the login/push steps |
| Branch is `main` | `if: github.ref == 'refs/heads/main'` on the same steps |

A PR — including one from a fork — never triggers a publish, regardless of how many approvals it has; only a merge to `main` does [Q3]. This mirrors `team.md`'s direct-to-`main` way of working: there is no separate release branch or tag-driven release gate for this workflow's deliverable.

## Tracked, not gated

Per `team.md`'s Testing Posture, these are measured and reported but do **not** block a merge or a publish:

| Metric | Where | Target |
|---|---|---|
| Line/branch coverage | `cargo-llvm-cov` (run manually / at Build and Test, not wired into this CI workflow) | 80%, target not hard floor — case-by-case acceptable below if test completeness is otherwise satisfactory |
| Transform benchmark (allocation count, latency) | `cargo bench --bench transform_bench` | No fixed regression threshold — bench numbers are inherently noisy across machines; tracked for trend, reported in `README.md`'s Performance section |

Coverage and benchmark checks are **not** added as CI jobs in this stage — consistent with the confirmed team practice that they remain human-reviewed signals rather than automated gates.

## Rollback / no-rollback

There is no deploy step in this pipeline — the `docker` job's furthest effect is publishing an immutable, SHA-tagged image to GHCR. A bad `latest` tag is remedied by pushing a corrected commit to `main` (which re-runs the pipeline and republishes `latest`), not by a rollback procedure; the SHA-tagged images already published are never overwritten or deleted, so any consumer pinned to a SHA tag is unaffected by a subsequent bad build. Deployment of the image to a running environment is out of this workflow's scope (confirmed at Ideation), so there is no rollback runbook to write here.
