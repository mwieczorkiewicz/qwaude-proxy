# Discovered Rules — role-coercion-proxy

> **Status: FINAL.** Populated only from explicit human-stated hard
> constraints surfaced during the Step 4 interview — never from a reviewer
> suggestion the interview didn't confirm as a hard constraint, and never
> from inference.

## Mandated

- ALWAYS commit changes directly to `main` in small logical chunks, using
  Conventional Commits message format — no feature branches, no
  squash-merge step (Q1).
- ALWAYS enable clippy's `unwrap_used`, `expect_used`, and `panic` lints
  (via a `[lints.clippy]` table or crate-root `#![deny(...)]`), at minimum
  scoped to the request-handling modules, so the build itself catches a
  stray `unwrap()`/`expect()`/`panic!()` rather than relying on review alone
  (Q6).
- ALWAYS run `cargo audit` as a required dependency-vulnerability check
  before merge (Q8).
- ALWAYS pass the inbound request's existing `Authorization` header through
  unchanged when forwarding to `VLLM_BASE_URL` — no separately configured
  static upstream credential (Q9).

## Forbidden

- NEVER use `.unwrap()`, `.expect()`, or `panic!()` on the request path.
  This was already established at ideation as a design requirement; the
  interview added the mechanical enforcement mechanism (the clippy
  deny-lints above) rather than leaving it as a review-only convention
  (Q6).
