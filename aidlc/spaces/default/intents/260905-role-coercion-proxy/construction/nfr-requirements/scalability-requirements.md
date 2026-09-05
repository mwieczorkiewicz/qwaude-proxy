# Scalability Requirements — qwaude-proxy (role-coercion-proxy)

## NFR3: Scalability

| ID | Requirement | Detail |
|---|---|---|
| NFR3.1 | Stateless design | The binary holds no in-process state that would prevent running multiple instances behind a future load balancer — consistent with Domain Design's value-object modeling (ADR-002): no entity has identity or lifecycle beyond a single request, and `ProxyConfig` is immutable after startup. This is a design property to preserve, not a deployment decision. |
| NFR3.2 | Concurrency handling | See Performance NFR1.4 — async, non-blocking I/O handles concurrent requests within a single process; no horizontal-scaling requirement is in scope for this workflow. |
| NFR3.3 | Growth projection | None — this is an internal R&D tool serving a modest, fixed-size engineering team, not a scaled product with a growth curve to plan capacity against (confirmed Q2). |
| NFR3.4 | Horizontal scaling / topology | Explicitly out of scope for this workflow's deliverable — deployment and infrastructure work (including any scaling topology, load balancer, or multi-instance orchestration) was confirmed out of scope in Ideation. NFR3.1's stateless property means a future workflow could scale horizontally without a redesign, but designing that topology is not this workflow's job. |

## Source

Q2 (`nfr-requirements-questions.md`); Domain Design ADR-002 (stateless/value-object modeling); Ideation scope-document.md (deployment/infra out of scope).

## Assumptions & Open Questions

None.
