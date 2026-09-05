# Decision Log — Ideation Phase — qwaude-proxy (role-coercion-proxy)

| # | Stage | Decision | Rationale | Source |
|---|---|---|---|---|
| 1 | Intent Capture | Problem framed as: vLLM crashes on mid-conversation `system`-role messages, breaking the Claude Code → adapter → Bifrost → vLLM pipeline; driven by wanting to run Claude Code against self-hosted Qwen-family models | Confirmed by project owner | [memory:M1] |
| 2 | Intent Capture | Customer is internal only — the engineering team/operator of this pipeline | Confirmed by project owner | [memory:M1] |
| 3 | Intent Capture | Success = zero tokenizer crashes + green test suite + clean clippy/fmt + benchmark showing negligible overhead vs. a naive baseline | Confirmed by project owner | [memory:M1] |
| 4 | Intent Capture | Trigger is R&D/experimentation, not an active production incident | Confirmed by project owner | [memory:M1] |
| 5 | Intent Capture | Solo project — project owner is sole stakeholder/decision-maker, no formal team or reporting cadence | Confirmed by project owner | [memory:M2] |
| 6 | Intent Capture | Workflow-selected scope (`qwaude-proxy`) confirmed as matching the intended product boundary | Confirmed by project owner | [memory:M1] |
| 7 | Intent Capture (review) | Product-lead advisory review returned READY with 3 findings (R-01 benchmark threshold, R-02 "production traffic" wording, R-03 root-cause hedging) | Advisory findings accepted at the gate | [memory:M1] |
| 8 | Scope Definition | Full spec is the MVP; no smaller v0 | Confirmed by project owner | [memory:M3] |
| 9 | Scope Definition | Every in-scope item is Must Have (MoSCoW); no Should/Could/Won't items | Confirmed by project owner | [memory:M3] |
| 10 | Scope Definition | Sequencing is risk-first: transform logic (highest design risk) built and tested before HTTP wiring, config/logging/shutdown/error-handling, then the full test/benchmark suite | Confirmed by project owner | [memory:M3] |
| 11 | Scope Definition | No hard deadlines — R&D initiative | Confirmed by project owner | [memory:M3] |
| 12 | Scope Definition | No additional out-of-scope items beyond the deployment/infra/team-formation exclusion | Confirmed by project owner | [memory:M3] |
| 13 | Approval & Handoff | Benchmark success criterion accepted as qualitative for now; a numeric threshold will be set once NFR Requirements/NFR Design has real measurements | Resolves R-01 from the Intent Capture review | [Q2] |
| 14 | Approval & Handoff | Go decision — proceed straight to Inception (Practices Discovery next) | Confirmed by project owner | [Q3] |

## Assumptions & Open Questions

None.
