# Reliability Requirements — qwaude-proxy (role-coercion-proxy)

## NFR4: Reliability

| ID | Requirement | Detail |
|---|---|---|
| NFR4.1 | Upstream failure handling | Fail fast — return 502/504 immediately with a small JSON error body on vLLM connection failure or timeout; no internal retry (confirmed Q3, so as not to duplicate retry logic Bifrost or Claude Code may already implement) |
| NFR4.2 | Graceful shutdown | On SIGINT/SIGTERM, drain in-flight requests — including long-lived streaming ones — before exiting; do not drop an in-progress response |
| NFR4.3 | No crash on malformed or unexpected input | Never panic or unwrap on the request path; malformed input and unexpected schema shapes return 400 with a JSON error body — mechanically enforced by the clippy deny-lints in `security-requirements.md` (NFR2.5) rather than left to review discipline alone |
| NFR4.4 | No availability SLA/SLO | Not applicable to this workflow's deliverable — deployment/infrastructure is out of scope, so there is no running, monitored instance to set an uptime target against. A future workflow that deploys this proxy would define availability targets at that time. |

## Recovery Objectives

Not applicable for the same reason as NFR4.4 — RTO/RPO/MTTR/MTBF are properties of a *deployed, monitored* system, and this workflow's deliverable is the binary plus its test/benchmark suite, not a running service.

## Source

Q3 (`nfr-requirements-questions.md`); `team-practices.md` (graceful shutdown, no-panic requirement already affirmed as project practices, restated here as formal NFRs); Ideation scope-document.md (deployment out of scope, hence no SLA).

## Assumptions & Open Questions

None.
