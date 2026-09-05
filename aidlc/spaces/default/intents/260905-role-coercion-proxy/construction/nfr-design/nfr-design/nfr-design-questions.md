# NFR Design — Questions

This is a single-unit project (no Units Generation ran), so these questions cover the whole proxy. They draw on the NFR Requirements artifacts and the architecture reviewer's findings from that stage.

## Already resolved (not a question)

The architecture reviewer flagged NFR2.7 (bounded request body size) as a devsecops addition that hadn't gone through a proper question. You've since confirmed directly: **10 MiB is the default, configurable via an environment variable.** This is now a settled design decision (see `security-design.md`), not open.

## Q1. How should `Transform` structurally support both `simd-json` (preferred) and `serde_json` (fallback)?

The reviewer flagged that NFR Requirements committed to this dual-library approach (Q6) without saying how one `Transform` component supports two distinct value-tree APIs.

A. Feature-flag based, compile-time choice: a Cargo feature (e.g. `simd`, enabled by default) selects the implementation; two parallel code paths behind a shared function signature/trait. No runtime detection — whichever feature the binary was built with is what runs. Recommended: SIMD support is effectively universal on x86_64/aarch64 server targets in practice, so a compile-time choice covers the real-world "unavailable" case (unusual build targets) without adding runtime-dispatch complexity to a proxy this size.
B. Runtime CPU-feature detection: a single binary detects SIMD support at startup (e.g. via `std::is_x86_feature_detected!`) and dispatches to whichever backend is available, without needing a recompile for different deployment targets
C. Simplify: use only `serde_json` for now; treat `simd-json` as a future optimization once the simpler path is proven correct
D. Not yet defined
X. Other (please specify)

[Answer]: B

## Q2. What timeout should trigger the fail-fast 504 when vLLM doesn't respond?

A. Connect timeout 5s, total request timeout 30s for non-streaming responses; for streaming responses, the timeout applies only to establishing the connection and receiving the first byte — not to total streaming duration, since a long legitimate completion shouldn't be killed mid-stream
B. Different values — I'll specify
C. Not yet defined
X. Other (please specify)

[Answer]: X - configurable via env varaible with 5s connect timeout and total req timeout of 30s for streaming. for streaming responses, the timeout applies only to establishing the connection and receiving the first byte — not to total streaming duration, since a long legitimate completion shouldn't be killed mid-stream

## Q3. Confirm the exact shape of the sanitized error response body (NFR2.6).

A. `{"error": {"message": "<short, generic, status-appropriate message>", "type": "<upstream_error | invalid_request | internal_error>"}}` — never includes raw upstream error text or a stack trace
B. A different shape — I'll specify
C. Not yet defined
X. Other (please specify)

[Answer]: A

## Consolidated Summary Confirmation

- Looks correct
- Request changes

[Answer]: Looks correct
