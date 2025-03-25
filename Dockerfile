FROM rust:1.85-slim as builder

WORKDIR /usr/src/app
COPY . .
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev \
&& apt-get clean \
&& rm -rf /var/lib/apt/lists/*
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/prompt-wrapper /app/prompt-wrapper
COPY --from=builder /usr/src/app/presets.yaml /app/presets.yaml

ENV RUST_LOG=info

EXPOSE 3000

CMD ["./prompt-wrapper"] 