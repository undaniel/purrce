FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# ponytail: dummy src para cachear deps; solo se rehace si cambia Cargo.toml
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && rm -rf src

COPY src ./src
COPY migrations ./migrations
RUN touch src/main.rs && cargo build --release

FROM alpine AS obscura-downloader

RUN apk add --no-cache curl tar && \
    ARCH=$(uname -m) && \
    case "$ARCH" in \
        x86_64)  OBSCURA_ARCH="x86_64" ;; \
        aarch64) OBSCURA_ARCH="aarch64" ;; \
        *) echo "Unsupported: $ARCH" && exit 1 ;; \
    esac && \
    curl -fsSL "https://github.com/h4ckf0r0day/obscura/releases/latest/download/obscura-${OBSCURA_ARCH}-linux.tar.gz" \
    | tar xz -C /usr/local/bin

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=obscura-downloader /usr/local/bin/obscura* /usr/local/bin/
COPY --from=builder /app/target/release/purrce-backend /usr/local/bin/

RUN useradd -r -u 1001 -s /bin/false purrce
USER purrce

EXPOSE 8080

CMD ["purrce-backend"]
