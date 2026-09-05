# Logical Infrastructure Components — qwaude-proxy (role-coercion-proxy)

This artifact bridges NFR Design to Infrastructure Design. Infrastructure Design does not run in this workflow (deployment/infrastructure work is confirmed out of scope in Ideation), so there is no downstream consumer to hand a deployment topology to — this document instead records the failure-domain and blast-radius picture for completeness, and for any future workflow that does add deployment.

## Component Inventory

| Component | Kind | Notes |
|---|---|---|
| `role-coercion-proxy` binary | Single process | Hosts both `ProxyServer` and `Transform` (Domain Design components.md) in one OS process; no separate services |

## Failure Domains

There is exactly one failure domain: the single process. `Transform` and `ProxyServer` are in-process function calls, not separate deployable units — a crash anywhere in the binary takes down the whole process (mitigated by the no-panic design in `reliability-design.md` NFR4.3, which is the actual defense here, not process isolation).

## Blast Radius

If the process crashes or becomes unresponsive: every in-flight request to this proxy fails; callers (Bifrost) see a connection failure rather than a clean 502/504. There is no redundancy or failover within this workflow's scope — a future deployment workflow would be where multi-instance redundancy, if wanted, gets designed (enabled by the stateless design in `scalability-design.md` NFR3.1, which is what makes that future option viable without a code change).

## Component Isolation / Shared Resources

No shared resources exist beyond what the OS provides to a single process (its own memory, its own TCP connections). There is no database, cache, or external state store for multiple instances to contend over.

## Support Perspective (AWS Platform)

No AWS service topology is being designed here — this document is intentionally minimal because Infrastructure Design is not part of this workflow's plan. A future workflow adding deployment would use this document's failure-domain/blast-radius analysis as its starting input.

## Source

`components.md` (Domain Design); `reliability-design.md`, `scalability-design.md` (this stage).

## Assumptions & Open Questions

None.
