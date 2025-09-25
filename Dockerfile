FROM rust:bookworm AS builder
WORKDIR /usr/src/chipay
COPY . .
RUN cargo install --path .

FROM debian:bookworm-slim
WORKDIR /chipay
COPY --from=builder /usr/src/chipay/*.html /chipay
COPY --from=builder /usr/local/cargo/bin/chipay /usr/local/bin/chipay
CMD ["chipay"]