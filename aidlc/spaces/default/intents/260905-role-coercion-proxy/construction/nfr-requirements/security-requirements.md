# Security Requirements — qwaude-proxy (role-coercion-proxy)

## NFR2: Security

| ID | Requirement | Detail |
|---|---|---|
| NFR2.1 | Authentication pass-through | Forward the inbound request's existing `Authorization` header unchanged; no separately configured upstream credential; no validation of the header's presence (affirmed in Practices Discovery Q9, confirmed again at Domain Design Q2) |
| NFR2.2 | No payload logging by default | Request/response body content never appears in logs unless verbose payload logging is explicitly enabled via config, for local debugging only |
| NFR2.3 | No persistence | Request and response bodies are processed entirely in-memory per request; nothing is written to disk, cached, or persisted beyond the lifetime of handling that request (confirmed Q5 — no additional data-protection requirement was identified beyond this) |
| NFR2.4 | Dependency vulnerability scanning | `cargo audit` runs as a required pre-merge check (affirmed in Practices Discovery) |
| NFR2.5 | Panic/unwrap prevention | Clippy deny-lints (`unwrap_used`, `expect_used`, `panic`) enabled on request-handling modules, mechanically enforcing "no panics on the request path" (affirmed in Practices Discovery) |
| NFR2.6 | Sanitized error responses | Error responses to the client never include raw upstream error bodies or internal stack traces — only a generic, status-appropriate message (e.g. `{"error": "upstream request failed"}`), consistent with the OWASP "avoid revealing internal details in error output" guidance |
| NFR2.7 | Bounded request body size | The proxy enforces a maximum accepted request body size (documented default, configurable via environment variable) to prevent unbounded memory growth from a single oversized request — this protects the zero-copy design's memory characteristics as much as it prevents a DoS vector. Flagged for the human to confirm or override at this stage's gate; no default was specified in the original task, so this NFR proposes a conservative starting default (e.g. 10 MiB) for Code Generation to implement, tunable without a design change. |

## Threat Model (STRIDE)

Applied to the `ProxyServer` component (the only component with a network-facing surface) and its two data flows: inbound (Bifrost/Claude Code → ProxyServer) and outbound (ProxyServer → vLLM).

| Threat | Assessment | Mitigation / Disposition |
|---|---|---|
| **S**poofing | Could an attacker impersonate Bifrost (inbound) or vLLM (outbound)? | Out of scope for this workflow — network-level trust boundary (internal network), not a code-level concern. No mitigation added here; deployment/infra work (where TLS and network policy would be addressed) is explicitly out of scope. |
| **T**ampering | Could a request/response be modified in transit? | TLS termination is assumed to be handled outside this proxy (internal network / a future deployment's concern) — the proxy itself adds no additional integrity check. Accepted risk given the internal-only, non-adversarial network context confirmed in Ideation. |
| **R**epudiation | Can an actor deny performing an action through this proxy? | No audit-logging requirement identified — this is an internal engineering tool with no compliance driver requiring non-repudiation (see Compliance Assessment below). |
| **I**nformation Disclosure | Can sensitive data leak? | Two concrete mitigations: NFR2.2 (no payload logging by default) and NFR2.6 (sanitized error responses, no raw upstream error content or stack traces reaching the client). |
| **D**enial of Service | Can the proxy be made unavailable or exhausted? | NFR2.7 (bounded request body size) directly addresses the memory-exhaustion vector for a proxy whose core design is allocation-sensitive. No rate limiting is proposed — out of scope given the confirmed modest internal load (NFR1.4) and no external/adversarial traffic. |
| **E**levation of Privilege | Can a user gain unauthorized access through this proxy? | Not applicable — the proxy has no authorization logic of its own; it is a transparent pass-through for whatever auth the caller presents (NFR2.1). |

## Compliance Assessment

No regulatory framework (GDPR, HIPAA, SOC 2, PCI-DSS) applies to this deliverable: it is an internal R&D engineering tool with no persistent storage of personal data, no payment processing, and no healthcare data. Claude Code conversations passing through the proxy may incidentally contain sensitive content (code, accidentally pasted credentials), but the proxy never persists or logs that content by default (NFR2.2, NFR2.3), which is judged sufficient given the internal network context (confirmed Q5). No compliance control matrix, PIA, or audit-trail requirement is warranted at this scope.

## Source

Q5 (`nfr-requirements-questions.md`); `team-practices.md` (NFR2.1, NFR2.4, NFR2.5 carry forward affirmed practices as formal NFRs); NFR2.6 and NFR2.7 are devsecops-agent recommendations per OWASP/STRIDE analysis, not yet human-confirmed — see the approval gate.

## Assumptions & Open Questions

- NFR2.7's specific default (10 MiB) is a starting proposal, not a confirmed number — Code Generation may adjust it if a smaller/larger bound proves more appropriate once real payload sizes are observed. [assumption]
