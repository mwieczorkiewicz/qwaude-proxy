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

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T13:02:03Z
**Iteration:** 1

This is an ADVISORY pass: findings below are ranked for the human to weigh at the approval gate, not a blocking gate.

### Findings

#### Major

1. **NFR2.6/NFR2.7 bypassed the stage's own question protocol.** Every other numeric/concrete target in this NFR pass (NFR1.1–1.3's percentages, NFR1.4's concurrency band, NFR4.1's fail-fast choice, NFR5.2–5.3's endpoint shapes) traces directly to a confirmed answer in `nfr-requirements-questions.md` (Q1–Q6). NFR2.6 (sanitized error responses) and NFR2.7 (bounded request body size, defaulted to 10 MiB) do not — they are devsecops-agent additions introduced for the first time in `security-requirements.md` itself, with no corresponding Q7 in the questions file. The stage definition's Step 4 makes ambiguity/gap resolution mandatory before proceeding ("If ANY ambiguity found: create follow-up questions and resolve before proceeding"), and Step 3 exists precisely to turn "unclear NFR areas" into `[Answer]:`-tagged questions. Folding a net-new, numeric design decision (the 10 MiB default) into prose under "Assumptions & Open Questions," to be accepted or rejected only via the stage's binary Approve/Request-Changes gate, gives the human no way to specify a different number without a full Request Changes round-trip — unlike every other value in this pass, which the human set directly by answering a lettered option. The underlying additions are reasonable on their technical merits (both are standard STRIDE-driven hardening for a public-facing-ish HTTP endpoint), but the process by which they entered this artifact is a unilateral call dressed as an "assumption" rather than a question.
   - *Location*: `security-requirements.md` > NFR2.6, NFR2.7 rows; `## Source`; `## Assumptions & Open Questions`.

2. **The confirmed dual JSON-library design (simd-json + serde_json fallback) has no stated integration architecture.** `tech-stack-decisions.md` commits, per the human's confirmed Q6 answer, to "prefer `simd-json` when SIMD support is available… falling back to `serde_json` when it isn't." `simd_json::owned::Value` and `serde_json::Value` are distinct types with distinct parsing/mutation APIs — they are not interchangeable at the call site. Yet `components.md` describes `Transform` as a single pure function with one documented behavior, and neither `components.md`/`decisions.md` nor `tech-stack-decisions.md` says how that single component is meant to support two backend value-tree types: a trait abstraction over both, two parallel code paths behind a `cfg`/feature flag, a build-time-only choice (no true runtime fallback), or something else. `tech-stack-decisions.md`'s rationale explains *why* each library was picked but not *how* they compose into one `Transform` implementation. A developer implementing this component has to invent that structural answer unguided.
   - *Location*: `tech-stack-decisions.md` > "JSON Handling: simd-json with serde_json fallback"; cross-referenced against `components.md` > `Transform`.

#### Minor

3. **NFR2.7's 10 MiB default is unvalidated against this proxy's actual payload shape.** The proxy forwards full Claude Code / Bifrost chat-completion request bodies, which can legitimately grow large with long conversation histories or big context windows. The artifact already flags the number as a starting assumption, which is good practice, but does not note the risk that a too-small default could reject legitimate large-context sessions — worth a line acknowledging that trade-off explicitly rather than treating 10 MiB as safely conservative in both directions.
   - *Location*: `security-requirements.md` > NFR2.7; `## Assumptions & Open Questions`.

4. **traceability.json's self-established NFR{1-5} base IDs extend the stage's documented schema without an explicit sensor check.** The stage file's own example schema for `traceability.json` assumes `upstream_ids` are inherited from an existing inception artifact; here they are minted by this stage itself because `requirements.md` was skipped. The `"notes"` field explains this clearly and the substitution is internally consistent across all six other artifacts (every `NFRx.y` reference resolves to a row this pass actually defines), so this is not a defect — but it is worth the human confirming the `traceability` sensor (declared in this stage's frontmatter) tolerates self-declared upstream IDs rather than expecting them pre-existing.
   - *Location*: `traceability.json` > `notes`, `upstream_ids`.

### Cross-Artifact Consistency Checks (passed)

- Every `NFRx.y` ID referenced in `traceability.json`'s `target` fields resolves to an actual row in the six requirement files; no dangling or missing IDs.
- Performance targets NFR1.1–1.3 are verbatim restatements of the confirmed Q1 answer (option A) — no invented numbers.
- Security, reliability, and observability requirements sourced from Practices Discovery / Domain Design (NFR2.1, NFR2.4, NFR2.5, NFR4.1–4.3, NFR5.1) accurately restate the upstream artifacts without drift.
- STRIDE analysis's Spoofing/Tampering "out of scope, accepted risk" dispositions are consistent with ADR-003 (no header-presence validation) and the confirmed internal-network trust boundary; no contradiction found.
- No SLA/SLO or growth-projection targets are asserted (NFR3.3, NFR4.4) — correctly justified against deployment being out of scope, avoiding the classic "99.99% availability with no deployed instance" inconsistency this review class watches for.

### Validation Tool Results

No `validation_tools` field is declared in this stage's frontmatter; none were run. Consistency was checked manually by cross-referencing every `NFRx.y` ID across all seven artifacts and the upstream Q&A/inception files.

### Summary

The numeric performance targets are sound and traceable to confirmed interview answers, and the STRIDE/compliance analysis is well-reasoned and internally consistent. The two points a human should weigh before approving are (1) NFR2.6/NFR2.7 entering the artifact as unilateral devsecops additions rather than through the stage's own question-and-answer mechanism, despite introducing a genuinely new numeric decision (the 10 MiB default), and (2) the confirmed dual-JSON-library tech-stack decision leaving no stated plan for how `Transform` structurally supports two incompatible value-tree APIs — a gap a developer will hit directly during Code Generation.
