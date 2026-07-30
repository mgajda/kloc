FROM docker.io/rust:1.85-slim-bookworm AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libc6-dev && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian12
COPY --from=builder /build/target/release/kloc /usr/local/bin/kloc
ENTRYPOINT ["/usr/local/bin/kloc"]
LABEL org.opencontainers.image.source=https://github.com/mgajda/kloc
LABEL org.opencontainers.image.description="Count lines of code and code complexity via universal AST parsing"
LABEL org.opencontainers.image.licenses=GPL-2.0-only
LABEL org.opencontainers.image.version=0.2.0
