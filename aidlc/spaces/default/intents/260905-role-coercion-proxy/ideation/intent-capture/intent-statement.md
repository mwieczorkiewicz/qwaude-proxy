# Intent Statement — qwaude-proxy (role-coercion-proxy)

## Problem Statement

vLLM's chat template raises `raise_exception("System message must be at the beginning")` whenever a `role: "system"` message appears anywhere in the `messages` array except index 0 [desc]. Something upstream — the Anthropic-to-OpenAI adapter and/or an agentic tool-context-refresh step — re-injects system-role messages mid-conversation on later turns, which crashes tokenization and breaks the `Claude Code → adapter → Bifrost → vLLM` pipeline [desc]. The underlying motivation is enabling Claude Code to run against self-hosted Qwen-family models through this pipeline [Q1].

## Target Customer

Internal only: the engineering team operating this self-hosted `Claude Code → Bifrost → vLLM` pipeline [Q2]. For implementation purposes, the customer is treated as a single internal user/operator [Q2]. The pain experienced is that agentic sessions crash or fail whenever a mid-conversation system message reaches vLLM [Q2].

## Success Metrics

Zero tokenizer crashes in production traffic caused by mid-conversation system-role messages [Q3]. This is verified by: the full test suite (unit tests, property-based tests, integration tests) passing; `cargo clippy -D warnings` and `cargo fmt --check` staying clean; and a criterion benchmark confirming the zero-copy transform adds negligible latency/allocation overhead compared to a naive full-deserialize/full-rebuild/full-reserialize baseline [Q3].

## Initiative Trigger

Research and development activity — this is proactive engineering work to enable running Claude Code against self-hosted models, not a response to an already-observed production incident [Q4].

## Initial Scope Signal

- **Workflow-selected scope**: `qwaude-proxy` — 13 of 33 stages (intent capture, scope definition, approval handoff, practices discovery, domain design, NFR requirements, NFR design, code generation, build-and-test, CI pipeline); no market research, no team formation, no UI, no deployment/infra automation [scope].
- **User-confirmed product boundary**: Confirmed as matching the workflow-selected scope — the entire deliverable is a single Rust proxy binary (code, tests, benchmarks, README); no deployment, infrastructure, or team-formation work is in scope [Q6].

## Assumptions & Open Questions

None.

## Review

**Verdict:** READY
**Reviewer:** aidlc-product-lead-agent
**Date:** 2026-09-05T11:15:19Z
**Iteration:** 1

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | intent-statement.md > Success Metrics | The benchmark criterion — "adds negligible latency/allocation overhead versus a naive full-deserialize baseline" — carries no numeric threshold (a %, an absolute latency bound, or an allocation-count delta). "Negligible" is not pass/fail testable as written, and this is the only quantitative-sounding success metric the whole initiative rests on; downstream (code-generation, NFR design, build-and-test) will have to invent a number the human never confirmed. | Add a concrete threshold to the criterion (e.g., "benchmark shows <X% latency delta and <Y bytes/op allocation delta vs. baseline"), sourced from a follow-up confirmation with the human — or explicitly tag it `[assumption]` under Assumptions & Open Questions if no number can be confirmed at this stage. | New |
| R-02 | Minor | intent-statement.md > Success Metrics | The metric "Zero tokenizer crashes in production traffic" sits alongside an Initial Scope Signal that excludes all deployment/infra work — there is no production environment or monitoring artifact in scope to ever observe "production traffic" against. The phrase is sourced verbatim from the confirmed Q3 answer, so it is not invented, but the juxtaposition could mislead a later stage into thinking a production observability deliverable is implied. | Consider a short clarifying note that this is a design-time/test-suite proxy for the eventual production goal, not a monitored SLO within this workflow's scope — no re-confirmation needed since the wording is user-sourced. | New |
| R-03 | Minor | intent-statement.md > Problem Statement | The causal chain "the Anthropic-to-OpenAI adapter and/or an agentic tool-context-refresh step re-injects system-role messages mid-conversation" is presented as established fact from `[desc]` rather than explicitly flagged as a hypothesis, though the "and/or" already hedges the exact culprit. Per the ideation phase guardrail (label uncertain claims as "hypothesis" or "assumption"), an unconfirmed root-cause mechanism read by a non-technical stakeholder could be taken as verified diagnosis. | No action required to unblock — cosmetic; a future revision could add "(hypothesized cause)" before the mechanism description. | New |

### Summary

Both artifacts are well-grounded: every substantive claim carries a source tag, both `## Assumptions & Open Questions` sections are honestly empty, and the stakeholder map correctly reflects the solo-project answer without inventing roles. The one point worth the human's attention is R-01 — the benchmark success criterion is not numerically testable as written, which could push an undefined threshold decision downstream. Advisory only; no blocking issues found.
