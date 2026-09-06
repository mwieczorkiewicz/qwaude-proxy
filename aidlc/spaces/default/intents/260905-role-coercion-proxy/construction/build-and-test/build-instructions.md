# Build Instructions — qwaude-proxy

## Prerequisites

- Rust toolchain (stable channel; edition 2021). Install via [rustup](https://rustup.rs/) if not already present.
- `cargo-audit` (`cargo install cargo-audit`) — required for the dependency-vulnerability check (NFR2.4).
- `cargo-llvm-cov` + the `llvm-tools-preview` rustup component, optional, only needed to reproduce the coverage percentage locally (`rustup component add llvm-tools-preview && cargo install cargo-llvm-cov`).

No external services, databases, or environment variables are required to build or run the test suite — all upstream dependencies (vLLM) are mocked in tests via `wiremock` or a hand-rolled mock server in `tests/common/mod.rs`.

## Dependency installation

```bash
cargo fetch
```

Pulls every pinned dependency in `Cargo.lock` (250 crates). No `npm install`/pip-equivalent step exists for this project.

## Build commands

```bash
cargo build --all-targets
```

Compiles the binary (`src/main.rs`), the library (`src/lib.rs`), every test target under `tests/`, and the benchmark target under `benches/`.

## Build verification

```bash
cargo build --release
```

Builds the optimized release binary at `target/release/qwaude-proxy` — the same profile (`opt-level = 3`, `lto = true`) that would ship. This also exercises the `[profile.release]` settings in `Cargo.toml`.

## Troubleshooting common build issues

| Symptom | Likely cause | Fix |
|---|---|---|
| `error: package `cargo-audit` is not installed` | `cargo-audit` missing | `cargo install cargo-audit` |
| `cargo-llvm-cov: command not found` | coverage tooling not installed | `rustup component add llvm-tools-preview && cargo install cargo-llvm-cov` (only needed to reproduce the coverage number; not required for `cargo build`/`cargo test`) |
| Build succeeds but `simd-json` path behaves unexpectedly | SIMD feature detection is a runtime check, not compile-time | No action needed — `qwaude_proxy::transform::simd_json_available()` falls back to `serde_json` automatically on architectures without the required SIMD instructions; both paths are tested |
| `cargo audit` reports a new advisory | A RustSec advisory was published after this workflow's Code Generation pass | Review the advisory; update the affected dependency in `Cargo.toml`/`Cargo.lock` if a fix is available, or document an accepted-risk exception if not |
