ARG BINARY_NAME_DEFAULT=reconned-instagram
ARG MY_GREAT_CONFIG_DEFAULT="someconfig-default-value"

FROM clux/muslrust:stable AS builder
RUN groupadd -g 10001 -r dockergrp && useradd -r -g dockergrp -u 10001 dockeruser
ARG BINARY_NAME_DEFAULT
ENV BINARY_NAME=$BINARY_NAME_DEFAULT
# Build the project with target x86_64-unknown-linux-musl

# Install ca-certificates in the builder stage and add the musl target
RUN apt-get update && apt-get install -y ca-certificates
RUN rustup target add x86_64-unknown-linux-musl

# Build dummy main with the project's Cargo lock and toml
# This is a docker trick in order to avoid downloading and building 
# dependencies when lock and toml not is modified.
COPY Cargo.lock .
COPY Cargo.toml .
RUN mkdir src \
    && echo "fn main() {print!(\"Dummy main\");} // dummy file" > src/main.rs
RUN set -x && cargo build --target x86_64-unknown-linux-musl --release
RUN ["/bin/bash", "-c", "set -x && rm target/x86_64-unknown-linux-musl/release/deps/${BINARY_NAME//-/_}*"]

# Now add the rest of the project and build the real main
COPY src ./src
RUN set -x && cargo build --target x86_64-unknown-linux-musl --release
RUN mkdir -p /build-out
RUN set -x && cp target/x86_64-unknown-linux-musl/release/$BINARY_NAME /build-out/

# Create a directory for certificates and copy them for the final image
RUN mkdir -p /build-out/etc/ssl/certs
RUN cp /etc/ssl/certs/ca-certificates.crt /build-out/etc/ssl/certs/

# Create a scratch based image
FROM scratch

# Copy the SSL certificates from the builder stage
COPY --from=builder /build-out/etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

ARG BINARY_NAME_DEFAULT
ENV BINARY_NAME=$BINARY_NAME_DEFAULT
ARG MY_GREAT_CONFIG_DEFAULT
ENV MY_GREAT_CONFIG=$MY_GREAT_CONFIG_DEFAULT

ENV RUST_LOG="error,$BINARY_NAME=info"
ENV SSL_CERT_FILE="/etc/ssl/certs/ca-certificates.crt"
ENV SSL_CERT_DIR="/etc/ssl/certs"

COPY --from=builder /build-out/$BINARY_NAME_DEFAULT /reconned-instagram

EXPOSE 8080
ENTRYPOINT ["/reconned-instagram"]
