FROM rust:1.85-slim-bookworm as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates i2pd netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

COPY --from=builder /app/target/release/kekulen /usr/local/bin/kekulen
COPY --from=builder /app/server ./server

ENTRYPOINT ["entrypoint.sh"]
CMD ["-p", "8585", "-d", "/app/data/my.dat", "--password", "password"]
