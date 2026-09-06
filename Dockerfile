# syntax=docker/dockerfile:1

# --- Build stage -------------------------------------------------------
# Alpine's Rust image is natively musl-libc, so `cargo build` here needs no
# target flag and no separate musl-tools cross-compiler package -- that
# avoids a known incompatibility between Debian's `musl-gcc` wrapper and the
# assembly aws-lc-sys (rustls's crypto backend) compiles at build time.
# rustls (not OpenSSL) is why this can be a static, libc-only binary at all.
FROM rust:1-alpine AS builder

RUN apk add --no-cache musl-dev ca-certificates

WORKDIR /build
COPY . .

RUN cargo build --release

# --- Final stage ---------------------------------------------------------
# `scratch` -- no shell, no package manager, no libc: the smallest possible
# attack surface for a proxy with no need to exec anything else at runtime.
FROM scratch

# CA certificates, in case VLLM_BASE_URL is ever configured as https://.
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /build/target/release/qwaude-proxy /qwaude-proxy

# Documents the LISTEN_ADDR default (0.0.0.0:8080); does not itself publish
# the port -- that's the run command's job.
EXPOSE 8080

ENTRYPOINT ["/qwaude-proxy"]
