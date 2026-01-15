ARG RELEASAURUS_VERSION
################################################################################
# Build
################################################################################
FROM rust:1.92.0-alpine3.23 AS builder
ARG RELEASAURUS_VERSION
RUN apk add --update --no-cache curl \
  && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | sh \
  && cargo binstall releasaurus@$RELEASAURUS_VERSION \
  && releasaurus --version

################################################################################
# Final
################################################################################
FROM alpine:3.22
COPY --from=builder /usr/local/cargo/bin/releasaurus /usr/local/bin
ENTRYPOINT [ "releasaurus" ]
