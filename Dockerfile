# ============================================
# Vortex Atoms AI - Multi-stage Docker Build
# ============================================

# Stage 1: Build Frontend
FROM node:22-alpine AS frontend-builder

WORKDIR /app/frontend

# Copy package files
COPY frontend/package*.json ./

# Install dependencies
RUN npm ci

# Copy frontend source
COPY frontend/ .

# Build frontend
RUN npm run build

# Stage 2: Build Backend
FROM rust:1.96-slim AS backend-builder

# Install dependencies for tray feature (glib, gtk, webkit, xdo)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    libxdo-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency caching
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release 2>/dev/null || true

# Copy source code
COPY src/ src/
COPY knowledge/ knowledge/

# Build backend
RUN touch src/main.rs && cargo build --release --bin vortex_api

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash vortex

WORKDIR /app

# Copy backend binary
COPY --from=backend-builder /app/target/release/vortex_api /usr/local/bin/

# Copy frontend build
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Copy knowledge files
COPY knowledge/ /app/knowledge/

# Copy config
COPY vortex.json /app/

# Set ownership
RUN chown -R vortex:vortex /app

# Switch to non-root user
USER vortex

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/v1/health || exit 1

# Run the server
ENTRYPOINT ["vortex_api"]
CMD ["--host", "0.0.0.0", "--port", "8080"]
