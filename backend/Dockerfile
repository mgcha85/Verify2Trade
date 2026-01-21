# Build Stage
FROM rust:1.84-slim-bookworm as builder

WORKDIR /usr/src/app
COPY . .

# Install dependencies for DuckDB/Polars if needed (usually static, but duckdb might need libs)
RUN apt-get update && apt-get install -y pkg-config libssl-dev g++

RUN cargo build --release

# Runtime Stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/backend /app/backend
COPY config.yaml /app/
COPY .env /app/

# Data directory needs to be mounted
VOLUME /app/cryptodata

CMD ["./backend"]
