################################################################################
# Build
################################################################################
FROM rust:1.89.0-alpine3.22 as builder
RUN apk --update --no-cache add alpine-sdk openssl-dev openssl-libs-static
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src src
RUN cargo build --release

################################################################################
# Final
################################################################################
FROM alpine:3.22
RUN apk add --update --no-cache bash
COPY --from=builder /build/target/release/releasaurus /usr/local/bin
USER nobody
ENTRYPOINT [ "releasaurus" ]
