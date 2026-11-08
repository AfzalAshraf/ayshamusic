# Aysha's Music Player — Docker image
# Pure Rust std library, ~20MB RAM target.
# Build: docker build -t aysha-music .
# Run:   docker run -p 5050:5050 --device /dev/net/tun --cap-add NET_ADMIN aysha-music

FROM rust:1.75-slim AS builder
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    yt-dlp \
    mpv \
    bluez \
    pulseaudio \
    tailscale \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/music-server /app/music-server
EXPOSE 5050
ENTRYPOINT ["/app/music-server"]
