# syntax=docker/dockerfile:1
#
# Builds the server binaries (UI embedded). This image doesn't run the server:
# it needs the host's audio devices and keyboard input, so run the binary natively.
#
#   docker build --target artifacts --output type=local,dest=dist .
#
# Produces dist/soundboard-server.exe (Windows) and dist/soundboard-server (Linux).

FROM node:24-bookworm-slim AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

FROM rust:1-bookworm AS build
RUN apt-get update \
    && apt-get install -y --no-install-recommends libasound2-dev pkg-config gcc-mingw-w64-x86-64 \
    && rm -rf /var/lib/apt/lists/* \
    && rustup target add x86_64-pc-windows-gnu
ENV CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY server/ server/
COPY --from=web /src/web/dist web/dist

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p soundboard-server \
    && cargo build --release --locked -p soundboard-server --target x86_64-pc-windows-gnu \
    && mkdir -p /out \
    && cp target/release/soundboard-server /out/ \
    && cp target/x86_64-pc-windows-gnu/release/soundboard-server.exe /out/

FROM scratch AS artifacts
COPY --from=build /out/ /
