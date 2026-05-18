# ── Build stage (always runs on the builder's native architecture) ─────────────
#
# Cross-compilation strategy: cargo-zigbuild uses Zig as a C cross-compiler,
# which handles the vendored Lua 5.4 (C source) and the Rust linker for both
# x86_64-unknown-linux-musl and aarch64-unknown-linux-musl — no QEMU needed.
FROM --platform=$BUILDPLATFORM rust:slim-bookworm AS builder

ARG BUILDARCH
ARG TARGETARCH
ARG ZIG_VERSION=0.14.0
ARG CARGO_ZIGBUILD_VERSION=0.19.8

# cmake: needed to build vendored Lua 5.4 from source.
# wget:  download Zig and cargo-zigbuild pre-built binaries.
RUN apt-get update && apt-get install -y --no-install-recommends cmake wget \
    && rm -rf /var/lib/apt/lists/*

# Install Zig — architecture-aware so this also works when the builder is arm64
# (e.g. Apple Silicon or a native arm64 CI runner).
RUN case "${BUILDARCH}" in \
        amd64) ZIG_ARCH=x86_64  ;; \
        arm64) ZIG_ARCH=aarch64 ;; \
        *) echo "Unsupported build arch: ${BUILDARCH}" >&2; exit 1 ;; \
    esac \
    && wget -qO /tmp/zig.tar.xz \
        "https://ziglang.org/download/${ZIG_VERSION}/zig-linux-${ZIG_ARCH}-${ZIG_VERSION}.tar.xz" \
    && tar -xJf /tmp/zig.tar.xz --strip-components=1 -C /usr/local/bin \
        "zig-linux-${ZIG_ARCH}-${ZIG_VERSION}/zig" \
    && rm /tmp/zig.tar.xz

# Install cargo-zigbuild from its own pre-built static binary (no recompilation).
RUN case "${BUILDARCH}" in \
        amd64) ZBUILD_ARCH=x86_64-unknown-linux-musl  ;; \
        arm64) ZBUILD_ARCH=aarch64-unknown-linux-musl ;; \
    esac \
    && wget -qO- \
        "https://github.com/rust-cross/cargo-zigbuild/releases/download/v${CARGO_ZIGBUILD_VERSION}/cargo-zigbuild-v${CARGO_ZIGBUILD_VERSION}.${ZBUILD_ARCH}.tar.gz" \
    | tar -xz -C /usr/local/cargo/bin/

# Map Docker architecture names to Rust musl target triples.
RUN case "${TARGETARCH}" in \
        amd64) echo "x86_64-unknown-linux-musl"  ;; \
        arm64) echo "aarch64-unknown-linux-musl" ;; \
        *) echo "Unsupported target arch: ${TARGETARCH}" >&2; exit 1 ;; \
    esac > /rust-target

RUN rustup target add "$(cat /rust-target)"

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

# cargo-zigbuild sets CC_<target>=zig-cc so that the cc crate compiles
# vendored Lua with the correct cross target automatically.
RUN cargo zigbuild --release --locked --target "$(cat /rust-target)" \
    && cp "target/$(cat /rust-target)/release/matrix-webhook" /matrix-webhook

# ── Runtime stage (distroless — no shell, no package manager) ─────────────────
#
# gcr.io/distroless/static-debian12 ships ca-certificates and nothing else.
# The binary is fully static (musl) so no glibc or libssl are needed.
# The :nonroot tag runs as uid 65532 by default.
FROM gcr.io/distroless/static-debian12:nonroot

WORKDIR /app
COPY --from=builder /matrix-webhook /matrix-webhook
COPY --chown=65532:65532 formatters/ /app/formatters/

EXPOSE 4785
ENTRYPOINT ["/matrix-webhook"]
