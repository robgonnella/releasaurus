################################################################################
# Build
################################################################################
FROM rust:1.90.0-alpine3.22 AS builder
RUN apk --update --no-cache add alpine-sdk openssl-dev openssl-libs-static
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src src
COPY scripts scripts
RUN cargo build --release

################################################################################
# Final
################################################################################
FROM alpine:3.22
RUN apk add --update --no-cache bash openssl ca-certificates
COPY --from=builder /build/target/release/releasaurus /usr/local/bin
ENTRYPOINT [ "releasaurus" ]
