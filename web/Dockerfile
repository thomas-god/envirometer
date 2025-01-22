# Build layer
FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml /app
COPY src/ /app/src
COPY migrations/ /app/migrations

RUN cargo build --release

# Prod layer
FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/web /
COPY --from=builder /app/migrations /migrations

CMD ["./web"]

# Sleep for debugging
# CMD ["tail", "-f", "/dev/null"]