# CI Pipeline — Questions

`.github/workflows/ci.yml` already exists — Code Generation built it proactively (Step 11), per `team.md`'s affirmed Testing Posture that test/lint/audit checks be "build-enforced... not left to manual discipline alone." These questions confirm it's adequate rather than designing one from scratch.

## Q1. Is GitHub Actions the right CI tool, and is the existing workflow's job structure (fmt+clippy+test in one job, `cargo audit` in a separate job) adequate?

A. Yes — GitHub Actions is correct (this is a GitHub-hosted repo), and the two-job split is fine as-is
B. No — different CI tool or job structure needed, I'll specify
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q2. The workflow triggers on push to `main` and on pull requests. Given the affirmed practice of committing directly to `main` (no feature branches), is the `pull_request` trigger still worth keeping?

A. Yes — keep both triggers; harmless if unused today, and ready if the team ever does open a PR (e.g. from a fork, or a future contributor)
B. No — remove the `pull_request` trigger since it's never actually used
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Q3. Are any artifact repositories (container registry, package registry) needed for this workflow?

A. No — deployment/infrastructure is out of scope for this workflow's deliverable; there's nothing to publish anywhere
B. Yes — I'll specify what and where
C. Not yet defined
X. Other (please specify)

[Answer]: B - I would like to use ghcr for storing my OCI contaienrs, ideally with test coverage (e2e, compilation and such). distroless/scratch if feasible

## Consolidated Summary Confirmation

- Q1: GitHub Actions is correct; the existing two-job split (fmt+clippy+test, cargo audit) is adequate as-is.
- Q2: Keep both the `push`-to-`main` and `pull_request` triggers.
- Q3: Add a container image build, publishing to GHCR (`ghcr.io/mwieczorkiewicz/qwaude-proxy`), using a `scratch`-based (distroless-equivalent) final image built via an Alpine/musl builder stage, with compilation coverage (the Docker build itself) and e2e coverage (running the built container and hitting `/health` and `/metrics` over real HTTP) gating the publish step. Publish is restricted to `push` events on `main` — never from a pull request.

Implemented as: `Dockerfile` (multi-stage, Alpine builder -> `FROM scratch`, 6.9MB image, verified buildable and runnable), `.dockerignore`, and a new `docker` job in `.github/workflows/ci.yml` (`needs: [test, audit]`) that builds the image on every push/PR, smoke-tests it, and pushes to GHCR only on push-to-`main`. All committed in `deb9dfa`.

- Looks correct
- Request changes

[Answer]: Looks correct
