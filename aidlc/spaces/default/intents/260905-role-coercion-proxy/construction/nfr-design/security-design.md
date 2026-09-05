# Security Design — qwaude-proxy (role-coercion-proxy)

## NFR2.1 — Authentication pass-through

**Design**: the request handler reads the inbound `Authorization` header value (if present) from the incoming axum request and clones it unchanged (a cheap `HeaderValue` clone, not a string reallocation of the body) into the outbound request builder before forwarding to `VLLM_BASE_URL`. No inspection, validation, or transformation of the value occurs. If the header is absent, none is sent upstream either — no default or synthesized credential is substituted (confirmed at Domain Design Q2, restated here as the concrete design).

## NFR2.2 / NFR2.3 — No payload logging, no persistence

**Design**: the `tracing` instrumentation at every request-handling call site logs only structural metadata (coercion count, indices, status codes, timing) — request and response body bytes are never passed as arguments to a `tracing::debug!`/`error!` call in the non-verbose path. No file, cache, or database write occurs anywhere in the request path; the entire request lifecycle is in-memory buffers scoped to that request's async task, dropped when the task completes.

## NFR2.4 / NFR2.5 — Dependency scanning, panic prevention

Already designed as build-time gates in `tech-stack-decisions.md` and `team-practices.md` (`cargo audit`, clippy deny-lints for `unwrap_used`/`expect_used`/`panic`); CI wiring for these checks is this workflow's CI Pipeline stage, not re-designed here.

## NFR2.6 — Sanitized error responses

**Design**: a single `ErrorResponse` struct is the only shape ever serialized into an error body, confirmed by Q3:

```rust
// pseudocode - interface-level, not implementation-ready
#[derive(Serialize)]
struct ErrorResponse<'a> {
    error: ErrorDetail<'a>,
}
#[derive(Serialize)]
struct ErrorDetail<'a> {
    message: &'a str, // short, generic, status-appropriate
    r#type: &'a str,  // "upstream_error" | "invalid_request" | "internal_error"
}
```

`ProxyError`'s `IntoResponse` implementation (axum's error-to-response trait) is the single place that constructs this struct, mapping each error variant to a status code and a fixed, generic message string — it never interpolates raw upstream response bytes, the underlying `std::error::Error` debug output, or a stack trace into the body. This is enforced structurally: nothing else in the codebase constructs an HTTP error response.

## NFR2.7 — Bounded request body size

**Design**: a default 10 MiB limit, configurable via an environment variable (e.g. `MAX_REQUEST_BODY_SIZE`), enforced via axum's `DefaultBodyLimit` tower layer wrapping the `POST /v1/chat/completions` route. A request whose body exceeds the limit is rejected with `413 Payload Too Large` **before** the body is read into memory for parsing — this protects the zero-copy parse path from ever allocating a buffer for an oversized request, and directly resolves the architecture review's flagged gap (the limit is now human-confirmed, not a unilateral devsecops default).

## Threat Model (STRIDE) — Design-Level Disposition

Restating `nfr-requirements/security-requirements.md`'s STRIDE table with the concrete design mechanism for each disposition that has one:

| Threat | NFR Requirements Disposition | Design Mechanism |
|---|---|---|
| Spoofing | Out of scope (network trust boundary) | No change — not addressed at this layer |
| Tampering | Accepted risk (TLS assumed external) | No change — not addressed at this layer |
| Repudiation | No requirement | No change |
| Information Disclosure | NFR2.2, NFR2.6 | No payload logging (above) + sanitized `ErrorResponse` (above) |
| Denial of Service | NFR2.7 | `DefaultBodyLimit` layer, 10 MiB default (above) |
| Elevation of Privilege | Not applicable | No change — proxy has no authorization logic |

## Support Perspective (AWS Platform)

No IAM, KMS, Secrets Manager, or network-security-group design applies — this proxy handles secrets (the pass-through `Authorization` header) entirely in-process with no AWS service integration, consistent with deployment/infrastructure being out of scope for this workflow.

## Source

Q1, Q3 (`nfr-design-questions.md`); user confirmation of NFR2.7's 10 MiB default (mid-session, resolving the architecture reviewer's flagged gap from NFR Requirements); `security-requirements.md` (NFR2.1-2.7).

## Assumptions & Open Questions

None — both open items the architecture reviewer flagged at NFR Requirements (the body-size default and, via Q1 above, the JSON-library dispatch mechanism) are now resolved.

## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-09-05T13:16:27Z
**Iteration:** 1

This is an ADVISORY pass: findings below are ranked for the human to weigh at the approval gate, not a blocking gate.

### Findings

#### Major

None.

#### Minor

1. **Both prior-stage Major gaps are genuinely resolved, but citation hygiene across the two resolving files is sloppy enough to obscure that.** `security-design.md`'s own `## Source` line ("Q1, Q3") and its closing `## Assumptions & Open Questions` paragraph both claim credit for resolving "the JSON-library dispatch mechanism… via Q1 above" — but Q1 (runtime SIMD-detection dispatch, confirmed Option B) is neither discussed nor used anywhere in this file's body; it is `performance-design.md` that actually implements Q1 (with a concrete `parse_and_coerce` dispatch snippet). Symmetrically, `performance-design.md`'s `## Source` line cites "Q1, Q2" even though Q2 (the fail-fast timeout values) is never discussed in that file — Q2 is fully and correctly implemented in `reliability-design.md` instead, whose own `## Source` line correctly cites only "Q2". The substance is right in both cases (each Q is answered exactly once, in the correct file, with no contradiction), but the cross-file `## Source`/closing-paragraph citations read as copy-paste bleed between sibling files rather than an accurate per-file source list, and a reader skimming only `security-design.md` could wrongly conclude the JSON-dispatch architecture question was addressed there.
   - *Location*: `security-design.md` > `## Source`, closing `## Assumptions & Open Questions` paragraph; cross-referenced against `performance-design.md` > `## Source`, `## NFR1.1 / NFR1.2 / NFR1.3`.
   - *Suggested action (non-blocking)*: tighten each file's `## Source` line to the questions actually used in that file's own content, and drop the "via Q1 above" claim from `security-design.md`'s closing paragraph (or repoint it to `performance-design.md`).

2. **NFR2.7's resolution still bypasses the stage's own lettered-question mechanism, even though it is now genuinely human-confirmed.** The previous review's finding was two-part: (a) the number entered the record without going through Step 2/3's `[Answer]:` protocol, and (b) it was unresolved. This stage resolves (b) cleanly — `nfr-design-questions.md`'s "Already resolved (not a question)" section states the human confirmed "10 MiB is the default, configurable via an environment variable" directly, and `security-design.md` and `traceability.json` both restate that value consistently. But (a) is only partially addressed: the confirmation still arrives as a prose annotation outside the lettered `[Answer]:` structure used for Q1–Q3, rather than as a proper question with options the human selected from. This is a much smaller concern than the original finding (the number is now unambiguously human-approved, not a devsecops assertion dressed as an assumption), but it is worth the human noting that the stage's own process convention was worked around rather than followed a second time.
   - *Location*: `aidlc/spaces/default/intents/260905-role-coercion-proxy/construction/nfr-design/nfr-design-questions.md` > "Already resolved (not a question)"; `security-design.md` > NFR2.7.

### Prior-Stage Major Findings — Resolution Check

| Prior finding (from `security-requirements.md` review) | Status | Evidence |
|---|---|---|
| NFR2.6/NFR2.7 entered without going through the question protocol | **Resolved (substantively)** | NFR2.7: human-confirmed 10 MiB default, env-configurable, stated in `nfr-design-questions.md`'s "Already resolved" note and restated identically in `security-design.md`'s NFR2.7 section and `traceability.json`. NFR2.6: Q3 in `nfr-design-questions.md` gives a lettered, human-selected answer (Option A), and `security-design.md` implements it as a concrete `ErrorResponse`/`ErrorDetail` struct (≤15-line pseudocode) with `message`/`type` fields matching Q3's confirmed shape exactly, including the exact `upstream_error \| invalid_request \| internal_error` type enum. See Minor finding 2 above for the residual process-hygiene note on NFR2.7. |
| Dual JSON-library (simd-json/serde_json) integration architecture unstated | **Resolved** | Q1 in `nfr-design-questions.md` gives a lettered, human-selected answer (Option B — runtime CPU-feature detection). `performance-design.md` NFR1.1/1.2/1.3 implements this as a concrete `parse_and_coerce` runtime-dispatch function (8-line pseudocode, within the ≤15-line constraint) that branches on `simd_json_supported()` to `transform_simd`/`transform_serde`, both following the same targeted-mutation strategy. This directly answers the "how does one `Transform` component support two distinct value-tree APIs" gap the prior review raised. |

### Cross-Artifact Consistency Checks

- **Timeout values**: `reliability-design.md` NFR4.1 (5s connect / 30s total for non-streaming, no total deadline once streaming begins) matches `nfr-design-questions.md` Q2's confirmed answer verbatim, including the streaming-carve-out language.
- **`traceability.json` coverage**: all 24 `NFRx.y` IDs across the six upstream `nfr-requirements/*.md` files (NFR1.1–1.5, NFR2.1–2.7, NFR3.1–3.4, NFR4.1–4.4, NFR5.1–5.4) are present with a `status`; every `N/A` entry carries a concrete, checkable justification (e.g. NFR2.4/NFR2.5 point to CI/clippy mechanisms already designed elsewhere rather than silently dropping the ID; NFR3.3/NFR3.4/NFR4.4/NFR5.4 all correctly point to the same "deployment/infra out of scope" boundary already established at Ideation) — no dodge-style N/A found.
- **`logical-components.md`'s "no Infrastructure Design consumer" framing**: verified against `aidlc-state.md`'s composed plan — stage 3.4 (`infrastructure-design`) is explicitly listed under "Stages to Skip" and marked `SKIP` in the stage checklist, so the artifact's framing is accurate, not an invented excuse.
- **Code-snippet length**: every fenced code block across the seven design files is ≤15 lines (10, 8, and 8 lines respectively in `security-design.md`, `performance-design.md`, `reliability-design.md`; `scalability-design.md`, `observability-design.md`, and `logical-components.md` contain no code blocks), and all are explicitly marked "pseudocode"/"interface-level, not implementation-ready," consistent with this stage's own Constraints section.
- **NFR2.1 Authorization pass-through**: `security-design.md`'s design (clone the `HeaderValue` unchanged, no substitution when absent) is consistent with `security-requirements.md` NFR2.1 and the Practices Discovery/Domain Design Q2 confirmations cited there — no drift.

### Validation Tool Results

No `validation_tools` field is declared in this stage's frontmatter; none were run. Coverage and cross-reference checks were performed manually against all six upstream `nfr-requirements/*.md` files, `nfr-design-questions.md`, `aidlc-state.md`, and all seven produced artifacts.

### Summary

Both Major findings carried over from the `security-requirements.md` review are genuinely resolved with traceable evidence, not just asserted: NFR2.6/NFR2.7 now rest on human-confirmed answers (Q3 and the "Already resolved" note respectively) and concrete designs, and the dual-JSON-library dispatch mechanism has a stated runtime-detection architecture matching the confirmed Q1 answer. Remaining issues are cosmetic/process-hygiene only — misattributed `## Source` citations bleeding between `security-design.md` and `performance-design.md`, and NFR2.7's confirmation still arriving as a prose note rather than a lettered question — neither of which affects the substance a developer would build from.
