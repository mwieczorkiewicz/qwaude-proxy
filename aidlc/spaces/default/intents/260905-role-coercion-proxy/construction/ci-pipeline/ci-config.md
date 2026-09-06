# CI Pipeline Configuration — qwaude-proxy

## Sources

- [Q1], [Q2], [Q3] — `ci-pipeline-questions.md`
- [memory:M-team-way-of-working] — `team.md` § Way of Working (direct-to-`main` commits, Conventional Commits, no feature branches)
- [memory:M-team-testing-posture] — `team.md` § Testing Posture ("A CI workflow that runs `cargo test`, `cargo clippy -D warnings`, `cargo fmt --check`, and `cargo audit` on every push/PR — with no deploy step — is in-scope project tooling")
- Existing `.github/workflows/ci.yml`, built proactively during Code Generation (Step 11) and extended this stage with the `docker` job

## Tool: GitHub Actions

This is a GitHub-hosted repository (`github.com/mwieczorkiewicz/qwaude-proxy`), so GitHub Actions is the natural CI tool — no external CI system (Jenkins, CodeBuild) is in play [Q1].

## Trigger strategy

```yaml
on:
  push:
    branches: [main]
  pull_request:
```

The team commits directly to `main` [Q2, memory:M-team-way-of-working], so `pull_request` is not the primary path today, but the trigger is kept: harmless when unused, and ready if a fork or a future contributor opens a PR [Q2].

## Jobs

Three jobs, all on `ubuntu-latest`:

### `test` (fmt, clippy, test)

| Step | Command |
|---|---|
| Toolchain | `dtolnay/rust-toolchain@stable` with `rustfmt, clippy` components |
| Cache | `Swatinem/rust-cache@v2` |
| Format | `cargo fmt --check` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Test | `cargo test --all-targets` |

### `audit` (dependency vulnerabilities)

`rustsec/audit-check@v2` against `Cargo.lock`, using `secrets.GITHUB_TOKEN`. Runs in parallel with `test` — independent of it, no shared setup.

### `docker` (container build, smoke test, publish) — added this stage per [Q3]

```yaml
docker:
  needs: [test, audit]
  permissions:
    contents: read
    packages: write
```

Gated on `test` and `audit` both passing first — a broken build or a failed lint/test run never reaches the container stage. Steps, in order:

1. **`docker/setup-buildx-action@v3`** — enables the BuildKit-backed builder `docker/build-push-action` needs.
2. **`docker/metadata-action@v5`** computes the GHCR tags: `type=sha,format=long` (full commit SHA, always) and `type=raw,value=latest,enable={{is_default_branch}}` (only on `main`). Automatically lowercases `ghcr.io/${{ github.repository }}`, which GHCR requires.
3. **Build (compilation coverage)** — `docker/build-push-action@v6` with `push: false, load: true`, tagged with the metadata output. This runs the full multi-stage `Dockerfile` (Alpine/musl builder → `FROM scratch`) on every push and PR, not just on the publish path — a from-scratch `cargo build --release` is exercised every time CI runs.
4. **Smoke test (e2e coverage)** — runs the just-built image as a real container (`docker run`), polls `GET /health` for up to 15s, then asserts both `/health` and `/metrics` return `200` over real HTTP. Since the final image is `scratch` (no shell, no package manager), this is the only way to verify it actually starts and serves traffic — there's nothing inside the image itself to introspect.
5. **Publish (push-to-`main` only)** — `docker/login-action@v3` then `docker push` for each computed tag, gated by `if: github.event_name == 'push' && github.ref == 'refs/heads/main'`. Never runs on a PR — a PR may originate from a fork with no legitimate reason to publish a package under this repo's GHCR namespace. Reuses the image already built and smoke-tested in steps 3–4 (no rebuild), so what's published is provably the exact artifact that passed the e2e check.

### Registry: GHCR

`ghcr.io/mwieczorkiewicz/qwaude-proxy`, authenticated with the workflow's own `secrets.GITHUB_TOKEN` (`packages: write` permission scoped to the job) — no separate PAT or registry credential to manage [Q3].

## Image build details

See `Dockerfile` and `.dockerignore` at the repo root:

- **Builder**: `rust:1-alpine` (native musl-libc) — chosen over a Debian builder + `musl-tools` cross-compiler after the latter failed to compile `aws-lc-sys`'s assembly under `musl-gcc`'s `-m64` flag. Alpine's toolchain is natively musl, so `cargo build --release` needs no `--target` flag and no cross-compiler package.
- **Final stage**: `FROM scratch` — no shell, no package manager, no libc beyond what's statically linked. `rustls` (not OpenSSL) is what makes a libc-only static binary possible at all.
- **Result**: a 6.9MB image, verified to build and to serve `/health`/`/metrics` correctly (see `## Review` in `memory.md` for the smoke-test transcript).
- CA certificates are copied in from the builder stage, in case `VLLM_BASE_URL` is ever configured as `https://`.

## Alternatives considered

- **Debian + `musl-tools` cross-compilation builder**: rejected — fails to build `aws-lc-sys`'s assembly under `musl-gcc`.
- **`gcr.io/distroless/cc` or `distroless/static` as the final stage** instead of `scratch`: would add a non-root user and a minimal set of libc runtime pieces distroless provides; not needed here since the binary is fully static (musl) with no dynamic libc dependency, so `scratch` achieves the same "nothing to exploit" property with an even smaller image. Not adopted, but noted as the fallback if a future need (e.g. wanting `/etc/passwd` for a non-root UID, or debugging tools) arises.
- **Rebuild-then-push** (build once locally for smoke test, rebuild for the publish step): rejected in favor of build-once-tag-multiple/reuse — guarantees the published image is byte-identical to the one that passed the e2e smoke test, and avoids double compilation time in CI.
