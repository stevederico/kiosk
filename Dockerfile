# Base images are pinned by digest (tag kept for readability). Refresh with
# `docker pull <tag>` + `docker image inspect <tag> --format '{{index .RepoDigests 0}}'`.
FROM node:24.22.2-bookworm-slim@sha256:c86ecead9e55a5210ac1727de38a4ca840bf03dcaf34dc9749addbf383b1920c AS frontend

WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --ignore-scripts
COPY . .
RUN if find /app \( -name '.env' -o -name '.env.*' \) ! -name '.env.example' -print -quit | grep -q .; then \
      echo 'FATAL: .env file present in Docker build context — aborting'; \
      find /app \( -name '.env' -o -name '.env.*' \) ! -name '.env.example' -print; \
      exit 1; \
    fi
RUN npm run build

FROM rust:1.99.1-bookworm@sha256:9181b56c2a21ab1d5e73cffe04fc762426d30aa7fac3c91141dd033f49a7fc8b AS backend

# Use the image's pinned toolchain; rust-toolchain.toml says "stable", which
# would otherwise make rustup download whatever stable is current.
ENV RUSTUP_TOOLCHAIN=1.99.1
RUN apt-get update && apt-get install -y --no-install-recommends \
      libsqlite3-dev libcurl4-openssl-dev pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY backend/rust-toolchain.toml backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim@sha256:99c0b25c46d0214d5a2091092e7908907dcaecd6a962cde0ac700acee222b6f3

RUN apt-get update && apt-get install -y --no-install-recommends \
      libsqlite3-0 libcurl4 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --home /app --shell /usr/sbin/nologin skateboard

WORKDIR /app
COPY --from=frontend /app/dist ./dist
COPY --from=backend /build/target/release/skateboard-backend /usr/local/bin/skateboard-backend
COPY backend/config.json ./backend/config.json
# Checkout and the webhook read the plan list (stripeProducts) from here.
COPY src/constants.json ./src/constants.json

# SQLite lives in /app/backend/databases (Kiosk.db). Mount a volume there in
# production; the image itself never holds data. Empty on first boot: the app
# creates the schema and seeds the menu.
RUN mkdir -p /app/backend/databases \
    && chown -R skateboard:skateboard /app

USER skateboard
ENV NODE_ENV=production
ENV SKATEBOARD_BACKEND_DIR=/app/backend
EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8000/api/health >/dev/null || exit 1

CMD ["skateboard-backend"]
