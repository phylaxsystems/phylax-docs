FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

LABEL org.opencontainers.image.source=https://github.com/phylax-systems/phylax
LABEL org.opencontainers.image.licenses="AGPLv3"

# Builds a cargo-chef plan
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Set the build profile to be release
ARG BUILD_PROFILE=release
ENV BUILD_PROFILE $BUILD_PROFILE

# Install system dependencies
RUN apt-get update && apt-get -y upgrade 

# Builds dependencies
RUN cargo chef cook --profile $BUILD_PROFILE --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --profile $BUILD_PROFILE --locked --bin phylax

# ARG is not resolved in COPY so we have to hack around it by copying the
# binary to a temporary location
RUN cp /app/target/$BUILD_PROFILE/phylax /app/phylax

# Use debian:slim as the release image
FROM debian:stable-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app

# Copy phylax over from the build stage
COPY --from=builder /app/phylax /usr/local/bin

# Copy license
COPY LICENSE-* ./

EXPOSE 4269
ENTRYPOINT ["/usr/local/bin/phylax"]