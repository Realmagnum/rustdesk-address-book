# Stage 1: Build frontend
FROM node:22-alpine AS frontend
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci
COPY web/ ./
RUN npx vite build

# Stage 2: Build Rust backend
FROM rust:1-bookworm AS backend
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ src/
COPY migrations/ migrations/
COPY --from=frontend /app/web/dist/ web/dist/
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && cp target/release/rustdesk-address-book /app/rustdesk-address-book

# Stage 3: Minimal runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /app --shell /usr/sbin/nologin rustdesk
WORKDIR /app
COPY --from=backend /app/rustdesk-address-book .
COPY --from=backend /app/migrations/ migrations/
RUN mkdir -p /data && chown -R rustdesk:rustdesk /app /data
# Non-root: the process must not run as root.
USER rustdesk
EXPOSE 21114
ENV RUSTDESK_AB_DB_PATH=/data/db.sqlite3
VOLUME /data
CMD ["./rustdesk-address-book"]
