# Base images are pinned by multi-arch index digest (tag kept for
# readability). Each digest must have been first published on Docker Hub at
# least 7 days before it is pinned (supply-chain rule; see REPORT.md
# "Base images" for how the dates were checked). Look up a candidate with
# hub.docker.com/v2/repositories/library/<repo>/tags (`digest`,
# `tag_last_pushed`) and docker-library/repo-info history, never `docker pull`
# of a floating tag.
FROM node:24.22.0-bookworm-slim@sha256:91acba91b842bec0bf7ea6f5e09c644c7ccad72c4996ed9616d861d1c7cd60d4 AS frontend

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

FROM rust:1.99.1-bookworm@sha256:48b7f435b8fc6378ce43b44f15e3a47fd8919b2825f835c39bd4bd3da50af5b8 AS backend

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

FROM debian:bookworm-20261104-slim@sha256:734a72b3d6142a71993b4bc580ca01d8ca687f77a2c422d96a961d3376b179bc

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
