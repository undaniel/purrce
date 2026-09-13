# Stage 1: Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app/frontend

COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci --prefer-offline

COPY frontend/ ./
RUN npm run build

# Stage 2: Download Obscura (cached separately from app build)
FROM alpine AS obscura-downloader

ARG OBSCURA_VERSION=latest

RUN apk add --no-cache curl tar && \
    ARCH=$(uname -m) && \
    case "$ARCH" in \
        x86_64)  OBSCURA_ARCH="x86_64" ;; \
        aarch64) OBSCURA_ARCH="aarch64" ;; \
        *) echo "Unsupported: $ARCH" && exit 1 ;; \
    esac && \
    if [ "$OBSCURA_VERSION" = "latest" ]; then \
        URL="https://github.com/h4ckf0r0day/obscura/releases/latest/download/obscura-${OBSCURA_ARCH}-linux.tar.gz"; \
    else \
        URL="https://github.com/h4ckf0r0day/obscura/releases/download/${OBSCURA_VERSION}/obscura-${OBSCURA_ARCH}-linux.tar.gz"; \
    fi && \
    curl -fsSL "$URL" | tar xz -C /usr/local/bin

# Stage 3: Build backend
FROM rust:1.98-slim-bookworm AS backend-builder

# cmake/clang/perl are required to build BoringSSL (btls-sys), pulled in by the
# impersonating HTTP client (wreq).
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    build-essential \
    cmake \
    clang \
    libclang-dev \
    perl \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# ponytail: dummy src para cachear deps; solo se rehace si cambia Cargo.toml
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    cargo build --release && rm -rf src

COPY backend/src ./src
COPY backend/migrations ./migrations
RUN touch src/main.rs && cargo build --release

# Stage 4: Final image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=obscura-downloader /usr/local/bin/obscura* /usr/local/bin/
COPY --from=backend-builder /app/target/release/purrce-backend /usr/local/bin/purrce-backend
COPY --from=frontend-builder /app/frontend/build /app/frontend/build
COPY --from=backend-builder /app/migrations /app/migrations

RUN useradd -r -u 1001 -s /bin/false purrce && \
    chown -R purrce:purrce /app

USER purrce

ENV RUST_LOG=purrce_backend=info,tower_http=info
ENV FRONTEND_DIR=/app/frontend/build

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/api/health || exit 1

CMD ["purrce-backend"]
