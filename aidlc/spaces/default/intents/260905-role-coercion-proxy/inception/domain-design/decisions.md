# Architecture Decision Records — qwaude-proxy (role-coercion-proxy)

## ADR-001: Two-component decomposition (Transform, ProxyServer)

### Status
Accepted

### Date
2026-09-05

### Context
The proxy needs at least one component boundary between the pure message-transform logic and the HTTP-serving/forwarding machinery, since the team's affirmed practices already require `transform.rs` to be independently unit- and property-testable without a server. The open question was whether the remaining HTTP-serving responsibilities (routing, config, upstream forwarding, streaming, logging, graceful shutdown) should be one component or split further into inbound- vs. outbound-facing pieces.

### Decision
Two components: **Transform** (pure message-array role coercion, no I/O) and **ProxyServer** (everything else — HTTP routing, config, upstream forwarding including streaming pass-through, logging, graceful shutdown, error mapping). Confirmed by the project owner at the Domain Design interview (Q1).

### Consequences

**Positive**
- Matches the team's already-affirmed file layout (`transform.rs` isolated from `main.rs`) exactly — no new file the team hasn't approved.
- `ProxyServer` stays a single component whose responsibilities genuinely change together (a routing tweak routinely touches config or error handling in the same commit), consistent with "if two things always change together, they are one component."
- Avoids introducing a component whose only job is forwarding to another component — a named architectural red flag.

**Negative**
- `ProxyServer` is a comparatively large component doing several jobs at once (routing, config, forwarding, streaming, logging, shutdown). If any one of these grows substantially in complexity later, it may need to be split out at that point.

**Neutral**
- Code Generation may still organize `ProxyServer`'s internals into multiple source-level modules or files (e.g. separate handler and client-forwarding functions) — that is an implementation detail, not a component boundary, and is left to Code Generation's own judgment.

### Alternatives Rejected

**Three components** (`Transform`, `RequestRouter`, `UpstreamClient`): splitting `ProxyServer` into an inbound-handling component and an outbound-forwarding component. Rejected because `RequestRouter` would have owned inbound HTTP parsing with essentially nothing to do except hand off to `UpstreamClient` — the same thin-pass-through anti-pattern the two-component design specifically avoids. It would also require a third source file beyond the team's affirmed two-file layout, which the project owner did not want to introduce for a project this size.

---

## ADR-002: Wire-format shapes are Value Objects, not DDD Entities

### Status
Accepted

### Date
2026-09-05

### Context
Domain Design's component catalogue schema asks each component to declare the entities it owns. This proxy is stateless — it has no database, no persisted domain objects, and no object whose identity outlives a single request. The candidate data shapes (`ChatCompletionRequest`, `Message`, `ContentBlock`, `ProxyConfig`) needed a consistent modeling decision.

### Decision
Model `ChatCompletionRequest`, `Message`, and `ContentBlock` (owned by `Transform`) and `ProxyConfig` (owned by `ProxyServer`) as DDD **Value Objects** — defined by their attributes, not by a persistent identity — and record `identifier: n/a` for each in the catalogue rather than inventing an artificial identity field. `ProxyConfig` is the one exception with a lifecycle note: it is a single process-wide instance (loaded once at startup), not created per request, but it is still not a DDD Entity since nothing tracks it by identity across changes — there are no changes; it's immutable after startup.

### Consequences

**Positive**
- Honest about the system's actual shape: nothing here has a lifecycle, a persisted identity, or state that outlives one request (or, for `ProxyConfig`, one process).
- Keeps the catalogue schema's `entities:` field meaningful (documents ownership and shape) without forcing a false identity concept onto transient data.

**Negative**
- None — this is a descriptive choice, not a structural trade-off.

**Neutral**
- Functional Design (a skipped stage in this workflow) would normally add full field types/validation constraints on top of this ownership+shape catalogue; here, the equivalent detail is a Code Generation concern given the task's already-detailed field-level spec.

### Alternatives Rejected

Treating these as DDD Entities with a synthetic identifier (e.g., a generated request ID) was considered and rejected — it would misrepresent the system's actual statelessness and add a concept (request identity) that nothing in the spec requires or uses.

---

## ADR-003: Authorization header is forwarded unchanged, with no presence validation

### Status
Accepted

### Date
2026-09-05

### Context
Practices Discovery (Q9) already decided the proxy passes through the inbound request's `Authorization` header unchanged, with no separately configured upstream credential. Domain Design needed to decide the remaining behavioral question: what happens if that header is absent on the inbound request?

### Decision
`ProxyServer` forwards whatever `Authorization` header state it receives — present or absent — without validating its presence. Confirmed by the project owner at the Domain Design interview (Q2).

### Consequences

**Positive**
- Keeps `ProxyServer` free of an authentication policy it has no way to actually enforce (it never inspects the header's value) — validating presence-only would be a false sense of security.
- Whatever access-control decision Bifrost or vLLM makes based on the header's presence/absence is preserved end to end, since the proxy never interferes with it.

**Negative**
- A caller that forgets to set `Authorization` gets no early, proxy-level signal that something is likely misconfigured — the failure (if any) surfaces further downstream at vLLM instead.

### Alternatives Rejected

Rejecting requests with a missing `Authorization` header (400/401) was considered and rejected — the project owner judged it a false layer of validation, since the proxy has no way to check whether the header's *value* is actually valid; requiring only its presence would not meaningfully improve security.

---

## ADR-004: Minimal request validation — only the `messages` array is validated

### Status
Accepted

### Date
2026-09-05

### Context
The spec requires malformed input to return 400. Domain Design needed to fix the boundary of what counts as "malformed": only the fields the proxy actually reads and transforms (`messages`), or the full expected OpenAI request schema.

### Decision
`ProxyServer` validates only that the request body is valid JSON with a `messages` array shaped as the transform expects. Every other top-level field (`model`, `temperature`, etc.) passes through untouched and unvalidated, whatever its value or absence. Confirmed by the project owner at the Domain Design interview (Q3).

### Consequences

**Positive**
- Matches the proxy's actual job: it only reads and rewrites `messages`, so validating fields it never touches would create a maintenance burden (schema drift) with no corresponding benefit.
- Keeps the proxy forward-compatible with any OpenAI-schema field additions Bifrost or the adapter might start sending, without requiring a proxy update.

**Negative**
- A request missing a semantically required field like `model` will not be caught by this proxy — it will fail later, at vLLM, with a vLLM-specific error rather than this proxy's own clean 400.

### Alternatives Rejected

Validating the full expected OpenAI request schema (requiring `model` and other standard fields to be present) was considered and rejected as scope creep relative to what this proxy actually does — vLLM already validates the fields it depends on.
