FROM rust:1.63.0-slim-bullseye as base
RUN apt update -y
RUN apt install -y --no-install-recommends \
    ca-certificates \
    gcc \
    libc6-dev 
#   libpq-dev \
#    libasn1-8-heimdal \
#    libtasn1-6 \
#    libhdb9-heimdal \
#    nettle-dev \
#    libhogweed6

RUN apt clean -y

ENV USER=drasil
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"

FROM base as builder
WORKDIR /dvltath-build
ENV CARGO_HOME=/dvltath-build/.cargo
COPY ./Cargo.toml ./                       
COPY ./.cargo/config $CARGO_HOME/config
COPY ./.cargo/git $CARGO_HOME/git
COPY ./src ./src
RUN cargo build --release --target x86_64-unknown-linux-gnu --bin dvltath

FROM gcr.io/distroless/cc as dvltath
WORKDIR /dvltath
COPY --from=builder /dvltath-build/target/x86_64-unknown-linux-gnu/release/dvltath /usr/bin

COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

ENV RUST_LOG=info

USER drasil:drasil

CMD ["dvltath"]
