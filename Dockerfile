# ==============================================================================
# PolyQuant 5M - Multi-Stage Production Dockerfile (Linux)
# ==============================================================================

# Stage 1: Build Frontend SPA
FROM node:20-bookworm-slim AS frontend-builder
WORKDIR /app/frontend

COPY frontend/package*.json ./
RUN npm install

COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust Backend Binary
FROM rust:1.80-bookworm AS backend-builder
WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./backend/
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
COPY config/config.yaml ./config/config.yaml

WORKDIR /app/backend
RUN cargo build --release

# Stage 3: Minimal Secure Production Runner
FROM debian:bookworm-slim AS runner
WORKDIR /app

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    sqlite3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy backend binary
COPY --from=backend-builder /app/target/release/poly_quant_backend /app/poly_quant_backend
RUN chmod +x /app/poly_quant_backend

# Copy frontend assets
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Copy default config and setup persistent directories
COPY config/config.yaml /app/config/config.yaml
RUN mkdir -p /app/data /app/logs

# Environment
ENV HOST=0.0.0.0
ENV PORT=8080
ENV APP_CONFIG_PATH=/app/config/config.yaml
ENV FRONTEND_DIST_PATH=/app/frontend/dist
ENV RUST_LOG=info,poly_quant_backend=debug,tower_http=info

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -fsS http://127.0.0.1:8080/api/v1/health || exit 1

VOLUME ["/app/data", "/app/config"]

CMD ["/app/poly_quant_backend"]
