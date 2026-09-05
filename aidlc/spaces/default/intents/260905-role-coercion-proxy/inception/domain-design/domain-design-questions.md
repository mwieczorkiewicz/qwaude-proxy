# Domain Design — Questions

Requirements Analysis and User Stories were skipped for this workflow (the task's own spec already serves as the requirements), so these questions and the resulting component catalogue draw directly from `intent-statement.md`, `scope-document.md`, `intent-backlog.md`, and the affirmed `team-practices.md` instead. Most implementation-level choices the original spec explicitly delegated to the architect (with a request to justify the choice) are decided here as documented ADRs rather than re-asked — only genuine team-preference or behavior-shaping decisions are asked below.

## Q1. How many logical components should this proxy be broken into?

The team's affirmed practices already fix the file layout as `transform.rs` isolated from `main.rs`. The open question is whether `main.rs`'s responsibilities (HTTP routing, request forwarding, streaming pass-through, config, logging, shutdown) should be one logical component, or split further.

- Option A — **Two components**: `Transform` (the message-array role-coercion logic — pure, no I/O) and `ProxyServer` (everything else: HTTP routing, config, upstream forwarding including streaming, logging, shutdown, error mapping). Pros: matches the team's already-affirmed 2-file module layout exactly; avoids a "pass-through" component whose only job is forwarding to another component (a named architectural red flag). Cons: `ProxyServer` is doing several jobs at once (routing + forwarding + config + lifecycle).
- Option B — **Three components**: `Transform`, `RequestRouter` (inbound HTTP handling + invoking Transform), and `UpstreamClient` (outbound forwarding to vLLM, including streaming). Pros: separates inbound-facing concerns from outbound-facing concerns. Cons: `RequestRouter` risks being a thin pass-through to `UpstreamClient` (the same anti-pattern Option A avoids), and it doesn't match the team's affirmed 2-file layout — would need a 3rd source file the team hasn't approved.

A. Option A — two components (Transform, ProxyServer), matching the affirmed file layout
B. Option B — three components (Transform, RequestRouter, UpstreamClient)
C. Not yet defined
X. Other (please specify)

[Answer]: A. Two components (Transform, ProxyServer)

## Q2. Should the proxy validate the presence of an `Authorization` header before forwarding, or forward whatever is present (including nothing)?

The team already decided (Practices Discovery, Q9) that the proxy passes through the inbound request's existing `Authorization` header unchanged, with no separately configured upstream credential. This question is about what happens if that header is *missing entirely* on the inbound request.

A. Forward as-is, no validation — if the inbound request has no `Authorization` header, none is sent upstream either; vLLM/Bifrost's own auth behavior decides what happens next
B. Reject with 400/401 if `Authorization` is missing — the proxy requires every inbound request to carry an auth header, even though it doesn't inspect its value
C. Not yet defined
X. Other (please specify)

[Answer]: A. Forward as-is, no validation

## Q3. Should the proxy validate that the request body has other required top-level OpenAI-schema fields (e.g. `model`), or only validate what it actually needs to touch (the `messages` array)?

This shapes what counts as a "malformed input → 400" response. The transform only ever reads/writes the `messages` array and leaves every other top-level field untouched and round-tripped.

A. Minimal validation — only require valid JSON with a `messages` array shaped as expected; every other top-level field (model, temperature, etc.) is passed through untouched and unvalidated, whatever it is (or isn't)
B. Fuller validation — also require the standard OpenAI-schema fields (e.g. `model`) to be present, rejecting with 400 if they're missing, even though the proxy doesn't use them
C. Not yet defined
X. Other (please specify)

[Answer]: A. Minimal validation

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
