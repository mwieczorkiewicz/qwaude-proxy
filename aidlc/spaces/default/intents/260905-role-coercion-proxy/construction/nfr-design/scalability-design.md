# Scalability Design — qwaude-proxy (role-coercion-proxy)

## NFR3.1 — Stateless design

**Design**: no design work beyond what Domain Design already established (ADR-002) — every data shape `Transform` and `ProxyServer` handle is a value object scoped to one request, and `ProxyConfig` is loaded once at startup and never mutated. There is no shared mutable state between concurrent request-handling tasks (no global cache, no in-process session store), so no synchronization primitives (mutexes, locks) are needed on the request path — a design property worth preserving precisely because it costs nothing to keep and would cost a redesign to add later if violated.

## NFR3.2 — Concurrency

Restated from `performance-design.md` NFR1.4 — this is the same async, non-blocking design; scalability and performance converge on the same mechanism at this proxy's scope.

## NFR3.3 / NFR3.4 — Growth, horizontal scaling, topology

Not applicable — no design work is warranted. No growth projection exists (confirmed Q2 at NFR Requirements: modest, fixed internal load), and horizontal-scaling topology is explicitly out of scope for this workflow's deliverable (deployment/infrastructure work). NFR3.1's stateless property is what would make horizontal scaling straightforward if a future workflow needs it — no additional design decision is required now.

## Support Perspective (AWS Platform)

No auto-scaling groups, load balancers, or partitioning strategy applies — there is no infrastructure topology in this workflow's scope to design against.

## Source

`scalability-requirements.md` (NFR3.1-3.4); Domain Design ADR-002.

## Assumptions & Open Questions

None.
