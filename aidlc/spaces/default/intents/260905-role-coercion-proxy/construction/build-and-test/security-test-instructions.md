# Security Test Instructions — qwaude-proxy

Generated beyond the Standard strategy's default because real NFR security requirements exist (`nfr-requirements/security-requirements.md`, including a STRIDE threat model, and `nfr-design/security-design.md`).

## Test framework setup

No dedicated SAST/DAST tooling is warranted at this project's scope (per the devsecops assessment in `nfr-requirements/security-requirements.md`) — Rust's memory safety plus clippy's lint set already cover the realistic vulnerability classes for a small, internal, single-crate proxy. Security-relevant behavior is instead verified through the same integration-test harness as functional behavior (`wiremock` + the hand-rolled mock upstream), plus two static checks.

## How to run

```bash
# Dependency-vulnerability scan (NFR2.4)
cargo audit

# Panic/unwrap-prevention lint (NFR2.5) -- part of the standard clippy invocation
cargo clippy --all-targets -- -D warnings

# Security-relevant integration tests (subset of tests/integration.rs)
cargo test --test integration authorization_header_is_forwarded_unchanged
cargo test --test integration oversized_body_returns_413
cargo test --test integration malformed_json_body_returns_400_invalid_request
cargo test --test integration upstream_connection_refused_returns_502
cargo test --test integration upstream_timeout_returns_504

# Error-response sanitization unit tests (NFR2.6) -- inline in src/error.rs
cargo test --lib error::tests
```

## Expected coverage / targets

| Requirement | Check | Expected result |
|---|---|---|
| NFR2.1 — auth header pass-through, no validation | `authorization_header_is_forwarded_unchanged` | 200 OK (wiremock only matches if the header arrived unchanged) |
| NFR2.2 — no payload logging by default | `tests/logging.rs` (3 tests) | Captured log fields never contain payload content unless verbose logging is explicitly enabled |
| NFR2.3 — no persistence | Manual code review this stage: `grep -rn "std::fs::\|File::create\|File::open\|OpenOptions" src/*.rs` | Zero matches (confirmed at this stage) |
| NFR2.4 — dependency scanning | `cargo audit` | 0 advisories against the current `Cargo.lock` |
| NFR2.5 — panic/unwrap prevention | `Cargo.toml` `[lints.clippy]` + `cargo clippy -- -D warnings` | Clean; `unwrap_used`/`expect_used`/`panic` denied crate-wide |
| NFR2.6 — sanitized error responses | `src/error.rs` unit tests + the four status-code integration tests above | Every error body matches `{"error": {"message", "type"}}`, never raw upstream text or a stack trace |
| NFR2.7 — bounded request body size | `oversized_body_returns_413` | 413 with the sanitized error shape, before the body is fully buffered |

## STRIDE disposition verification

Restated from `nfr-requirements/security-requirements.md` and `nfr-design/security-design.md`'s STRIDE table — this stage does not re-run threat modeling, only verifies the mitigations that have a concrete, checkable mechanism:

| Threat | Mitigation | Verified by |
|---|---|---|
| Information Disclosure | No payload logging (NFR2.2) + sanitized error responses (NFR2.6) | Tests above |
| Denial of Service | Bounded request body size (NFR2.7) | `oversized_body_returns_413` |
| Spoofing, Tampering, Repudiation, Elevation of Privilege | Explicitly out of scope / accepted risk / not applicable (network trust boundary, no auth logic in this proxy) | No test — design-level disposition only, per `security-requirements.md` |

## Secrets handling

No hardcoded credentials exist anywhere in the crate (verified by the same `grep` pass above extended to credential-shaped strings — none found beyond the `Authorization` header pass-through, which never stores or logs the credential value it forwards).
