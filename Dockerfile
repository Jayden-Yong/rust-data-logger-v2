# Multi-stage build for optimized production image
FROM rust:1.89 as builder

# Install Node.js for frontend build
RUN curl -fsSL https://deb.nodesource.com/setup_18.x | bash - && \
    apt-get install -y nodejs

WORKDIR /app

# Copy source files
COPY . .

# Build frontend first
WORKDIR /app/web
RUN npm install && npm run build

# Build Rust backend
WORKDIR /app
RUN cargo build --release

# Production stage - minimal image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary and web assets (maintain same structure as local)
COPY --from=builder /app/target/release/ava-device-logger /app/
COPY --from=builder /app/web/build /app/web/build/

# Create directories for data persistence with proper permissions
RUN mkdir -p /app/data /app/catalogs /app/logs && \
    chmod 755 /app/data /app/catalogs /app/logs

# Set environment variables
ENV RUST_LOG=info
ENV DATABASE_URL=/app/data/data.db

# Expose port
EXPOSE 8080

# Health check endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Run the application
CMD ["./ava-device-logger"]