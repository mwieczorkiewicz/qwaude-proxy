**Collaborator:** aidlc-developer-agent

## Contribution

Reviewed the draft's `## Code Style` section against the scope document's
explicit module-boundary requirement (`transform.rs` isolated from
`main.rs`'s HTTP wiring, so the transform logic is unit/property-testable
without a server) and the required `src/`/`tests/`/`benches/` layout, from a
senior Rust developer's perspective.

**1. Formatter/linter choice — sufficient as a baseline, but the linter line
undersells what it needs to do.**

`cargo fmt` + `cargo clippy -D warnings` is the right idiomatic default and
matches `org.md`'s "language-default" fallback — no objection to keeping it.
But as currently worded, `-D warnings` only escalates clippy's *default*
lint groups (`correctness`, `suspicious`, `complexity`, `perf`, `style`) to
errors. `clippy::unwrap_used`, `clippy::expect_used`, and `clippy::panic`
live in clippy's **restriction** group, which is off by default and is not
pulled in by `-D warnings` against the default lint set. In other words: as
drafted, the linter will not catch a stray `.unwrap()` on the request path —
the exact failure mode the scope document calls out as a hard MVP
requirement ("No `unwrap()`/`panic!()` on the request path"). That
requirement would then rest on tests and code review alone, with no
mechanical backstop. This project should affirm one of:

- a `[lints.clippy]` table in `Cargo.toml` (Rust 1.74+, no attribute needed)
  setting `unwrap_used = "deny"`, `expect_used = "deny"`, `panic = "deny"`
  (with a scoped `#[allow(...)]` on the handful of `#[cfg(test)]` modules
  where `.unwrap()` is normal), or
- the equivalent `#![deny(clippy::unwrap_used, clippy::expect_used,
  clippy::panic)]` crate-root attribute.

Either is a small, one-time addition and turns a code-review convention into
a build-breaking check — worth affirming now rather than discovering the gap
after the first `.unwrap()` slips into `main.rs`'s error-mapping glue.

**2. Error-handling convention — not addressed in the draft, and it directly
touches the layer boundary the draft does call out.**

The draft's Testing Posture section notes "no panics/unwraps on the request
path" as a testable requirement, but `## Code Style` says nothing about how
errors should be represented or propagated. For a no-panic/no-unwrap request
path returning "appropriate HTTP status codes with a small JSON error body"
(scope document) across distinct failure classes (malformed input, upstream
timeout/failure — items 8 and 3 in the intent backlog), hand-rolled
`Result<T, String>` or ad-hoc enums without `std::error::Error` impls will
not scale past the first few match arms, and reaching for `anyhow` on the
request path specifically works against the requirement: `anyhow::Error` is
intentionally type-erased, so it cannot be pattern-matched to select a 400
vs. 502 vs. 504 status code without downcasting.

Recommend affirming a `thiserror`-based typed-enum convention, split at the
same boundary the draft already draws:

- A `TransformError` (or similar) type owned by `transform.rs`, describing
  only transform-time failures (unexpected content-block shape, etc.) —
  critically, this type must stay free of HTTP/transport concerns (no status
  codes, no response types) so `transform.rs` keeps its stated independence
  from `main.rs`'s HTTP wiring.
- A request/proxy-level error enum (e.g. `ProxyError`, likely in `main.rs`
  or a small `error.rs`) that composes `TransformError` plus
  upstream-forwarding failures, and is the *only* place that knows about
  HTTP status codes and the JSON error-body shape (via the chosen HTTP
  framework's response-conversion trait, once that framework is picked).
- `anyhow` (if used at all) confined to `main()`'s own startup/config-parsing
  path — a genuinely fail-fast, process-exit case — never the request path.

This keeps the error-type boundary aligned with the module boundary the
scope document already mandates, rather than letting a convenient
type-erased error leak the HTTP layer's concerns back into the transform
layer.

**3. File organization — the draft's module-layout note is correct but
incomplete for the required `tests/`/`benches/` layout.**

The one bullet on `transform.rs` vs. `main.rs` correctly identifies the
highest-value boundary from the scope document. Two Rust-specific
conventions worth affirming alongside it, since they affect how the very
first testing Bolt is structured:

- Unit and property tests for `transform.rs` (and any other pure-logic
  module) belong inline as `#[cfg(test)] mod tests { ... }` in the same
  file — standard Rust convention, and it is what makes "unit/property-
  testable without a server" concrete rather than aspirational.
- Integration tests against the mock upstream belong under the top-level
  `tests/` directory (each file there compiles as its own test binary), with
  shared mock-upstream setup code factored into `tests/common/mod.rs` (the
  `mod.rs`-under-a-subdirectory naming Rust needs so the shared file isn't
  itself picked up and run as a standalone test binary). Without this
  convention stated up front, the integration-test Bolt (backlog item 10)
  risks duplicating mock-server setup across files.
- `benches/` entries need `[[bench]] name = "..." harness = false` in
  `Cargo.toml` for criterion to run them (criterion supplies its own
  harness) — a one-line note now avoids a stumble when the benchmark Bolt
  (backlog item 11) starts.

**4. Naming conventions — no changes.** The Rust-idiomatic case conventions
(`snake_case`/`CamelCase`/`SCREAMING_SNAKE_CASE`) stated in the draft are
correct and settled; nothing to add.

**What I'd affirm now at practices-discovery vs. leave to Domain Design.**
The transform/HTTP layer-boundary *principle* (transform logic must not
depend on HTTP-transport types) and the error-type split that follows from
it belong here — they are cross-cutting "how we write every module" rules
Code Generation needs from Bolt 1, and retrofitting them after code exists
is expensive. The *full* module decomposition beyond `transform.rs` vs.
`main.rs` (whether config/logging/shutdown/error each get their own file or
live inline) is a design decision that depends on actual responsibilities
and belongs in Domain Design, not a practices document. One adjacent gap
worth flagging without recommending this stage resolve it: no HTTP framework
(axum / actix-web / warp+hyper) is named anywhere in the ideation artifacts,
and the error-handling convention above (response-conversion trait at the
proxy-error boundary) is easiest to state precisely once that choice is
made — Domain Design's job, not this stage's.

## Positions

- AGREE: `cargo fmt` + `cargo clippy -D warnings` as the formatter/linter
  baseline — idiomatic, zero-config, matches `org.md`'s language-default
  fallback and the scope document's stated CI gates.
- AGREE: the `transform.rs` isolated from `main.rs` module-layout bullet —
  correctly identifies the scope document's highest-value layer boundary and
  states it as a Code Style convention rather than deferring it entirely to
  design.
- AGREE: the Rust-idiomatic naming conventions as stated — correct and
  needs no interview time.
- OBJECT: "Linter: `cargo clippy -D warnings`" presented as sufficient on
  its own — it does not enable clippy's restriction-category lints
  (`unwrap_used`, `expect_used`, `panic`), so it will not mechanically catch
  the exact failure the scope document forbids on the request path;
  recommend affirming an explicit `[lints.clippy]` table (or crate-root
  `#![deny(...)]`) naming those lints.
- OBJECT: the Code Style section has no error-handling convention at all,
  despite the no-panic/no-unwrap request-path requirement needing one to be
  enforceable; recommend affirming a `thiserror`-typed-enum-per-layer
  convention (transform-level error type kept HTTP-agnostic, a separate
  request/proxy-level error type owning the status-code mapping), with
  `anyhow` — if used — confined to startup/config parsing only.
