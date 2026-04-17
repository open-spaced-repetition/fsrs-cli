FROM rust:1.95-slim AS builder

RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=builder /app/target/release/fsrs /usr/local/bin/fsrs

EXPOSE 7320

ENTRYPOINT ["fsrs"]
CMD ["serve", "--host", "0.0.0.0", "--port", "7320"]
