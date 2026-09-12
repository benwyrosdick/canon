# Relay-only image. Does not compile Bevy or ship the game client.
#   docker build -t canon-relay .
#   docker run --rm -p 3478:3478 canon-relay
# Published as ghcr.io/<github-user>/canon-relay
FROM rust:1.88-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --no-default-features --bin canon-relay \
    && strip target/release/canon-relay

FROM debian:bookworm-slim
RUN useradd --system --no-create-home --shell /usr/sbin/nologin canon
COPY --from=build /src/target/release/canon-relay /usr/local/bin/canon-relay
USER canon
EXPOSE 3478/tcp
ENV CANON_RELAY_ADDR=0.0.0.0:3478
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD bash -c 'echo > /dev/tcp/127.0.0.1/3478' || exit 1
ENTRYPOINT ["canon-relay"]
