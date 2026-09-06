<!-- INVARIANT: examples are single-line HTML comments so a fresh template parses to total=0 (MEMORY_EMPTY). Do NOT un-comment or split across lines. t100 guards this. -->
> This file is kept up to date automatically while the stage runs. Add observations at the review step, not by editing here directly.

## Interpretations
<!-- example: 2026-05-29T10:14:32Z — chose REST over GraphQL; the consuming team only needs CRUD, revisit if subscriptions land -->
2026-09-06T17:35:29Z — interpreted "distroless/scratch if feasible" [Q3] as "prefer scratch when the binary can be fully static"; confirmed rustls (not OpenSSL) makes that true here, so went straight to scratch rather than distroless/cc.
2026-09-06T17:35:29Z — interpreted "test coverage (e2e, compilation and such)" [Q3] as two concrete CI checks: the multi-stage `docker build` itself (compilation coverage, exercised on every push/PR) and a running-container smoke test against `/health` + `/metrics` (e2e coverage), rather than a separate test framework.

## Deviations
<!-- example: 2026-05-29T10:14:32Z — skipped the optional caching layer the stage prose suggested; the dataset is small enough that it adds risk -->
2026-09-06T17:35:29Z — first Dockerfile attempt (Debian builder + musl-tools cross-compiler) failed to compile `aws-lc-sys`'s assembly under `musl-gcc -m64`; switched the builder base to `rust:1-alpine` (native musl) instead of debugging the cross-compiler flag mismatch further — lower-risk and no loss of capability.

## Tradeoffs
<!-- example: 2026-05-29T10:14:32Z — picked TDD over BDD this run; the team is unit-first and the domain is well-understood -->
2026-09-06T17:35:29Z — publish step reuses the already-built-and-smoke-tested local image (re-tag + `docker push`) rather than rebuilding for the push step; costs a slightly more manual tag-push loop in the workflow YAML, buys a guarantee that the published image is byte-identical to the one that passed the e2e check.
2026-09-06T17:35:29Z — did not wire `cargo-llvm-cov` or the criterion benchmark into this CI workflow, consistent with `team.md`'s Testing Posture marking both as tracked/target rather than hard-gated; only fmt/clippy/test/audit/docker-build/smoke-test are build-enforced.

## Open questions
<!-- example: 2026-05-29T10:14:32Z — confirm the retention window with compliance before the next stage hardens the schema -->
None.
