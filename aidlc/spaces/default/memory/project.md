# Project-Level Rules

> Project-specific specialisation and corrections. Loaded after `org.md` and
> `team.md` as strict-additive guidance; contradictions with broader policy
> are rejected. Populated by practices-discovery and the self-learning loop.
>
> Use sparingly: most teams don't need a project layer. Reach for it
> only when this specific project needs stable, durable guidance beyond the
> team practice (for example, package-specific release checks or an additional
> regression suite for a legacy component).

## Way of Working

<!-- Project-specific specialisation. Example: -->
<!-- This monorepo requires package-scoped branch names and a package owner -->
<!-- review in addition to the team's normal merge policy. -->

## Walking Skeleton

<!-- Project-specific specialisation. Example: -->
<!-- The walking skeleton must exercise the legacy service adapter as well -->
<!-- as the new service boundary. -->

## Testing Posture

<!-- Project-specific specialisation. -->

## Deployment

<!-- Project-specific specialisation. -->

## Code Style

<!-- Project-specific specialisation. -->

## Tech Stack

<!-- Technology choices locked for this project. -->

## Decided

<!-- Decisions made in earlier stages that should not be re-asked. -->
<!-- Format: DECIDED: [decision] (Stage [slug], [date]) -->

## Scope Overrides

<!-- Custom scope rules for this project. -->

## Forbidden

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: NEVER [behavior] (affirmed [date]) -->
<!-- Example: NEVER throw exceptions across service layer boundaries (affirmed 2026-05-17) -->

- NEVER use `.unwrap()`, `.expect()`, or `panic!()` on the request path. (affirmed 2026-09-05)
This was already established at ideation as a design requirement; the (affirmed 2026-09-05)
interview added the mechanical enforcement mechanism (the clippy (affirmed 2026-09-05)
deny-lints above) rather than leaving it as a review-only convention (affirmed 2026-09-05)
## Mandated

<!-- Populated by practices-discovery affirmation gate. -->
<!-- Format: ALWAYS [behavior] (affirmed [date]) -->
<!-- Example: ALWAYS use Result<T,E> for fallible operations in service layer (affirmed 2026-05-17) -->

- ALWAYS commit changes directly to `main` in small logical chunks, using (affirmed 2026-09-05)
Conventional Commits message format — no feature branches, no (affirmed 2026-09-05)
squash-merge step (Q1). (affirmed 2026-09-05)
- ALWAYS enable clippy's `unwrap_used`, `expect_used`, and `panic` lints (affirmed 2026-09-05)
(via a `[lints.clippy]` table or crate-root `#![deny(...)]`), at minimum (affirmed 2026-09-05)
scoped to the request-handling modules, so the build itself catches a (affirmed 2026-09-05)
stray `unwrap()`/`expect()`/`panic!()` rather than relying on review alone (affirmed 2026-09-05)
(Q6). (affirmed 2026-09-05)
- ALWAYS run `cargo audit` as a required dependency-vulnerability check (affirmed 2026-09-05)
before merge (Q8). (affirmed 2026-09-05)
- ALWAYS pass the inbound request's existing `Authorization` header through (affirmed 2026-09-05)
unchanged when forwarding to `VLLM_BASE_URL` — no separately configured (affirmed 2026-09-05)
static upstream credential (Q9). (affirmed 2026-09-05)
## Corrections

<!-- Project-specific corrections from human feedback. -->
<!-- Format: NEVER/ALWAYS [behavior] (learned [date]) -->
