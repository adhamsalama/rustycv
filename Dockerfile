# syntax=docker/dockerfile:1

# The editor. Built first and separately: it changes on its own schedule, and
# nothing in the Rust build depends on it.
FROM node:22-bookworm-slim AS web
WORKDIR /web
RUN corepack enable && corepack prepare pnpm@12.3.4 --activate
# Dependencies before sources, so editing a component does not reinstall them.
COPY web/package.json web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
RUN pnpm build

# The server. Templates, fonts and icons are `include_str!`/`include_bytes!`d
# into the binary, so they have to be here at build time even though nothing
# reads them from disk afterwards.
FROM rust:1.95-bookworm AS server
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY templates templates
COPY assets assets
COPY migrations migrations
# The seed binary embeds the reference resume; cheap to carry, confusing to omit.
COPY fixtures fixtures
RUN cargo build --release -p rustycv-server --bin rustycv-server

# What ships: one binary, the built editor, and nothing else. No fonts, no
# Typst packages, no toolchain — the render path only ever reads memory.
FROM debian:bookworm-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libcap2-bin \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --system --uid 10001 --no-create-home rustycv \
 && mkdir /data \
 && chown rustycv:rustycv /data

COPY --from=server /src/target/release/rustycv-server /usr/local/bin/rustycv-server
COPY --from=web /web/dist /srv/web/dist

# 80 is a privileged port and this image does not run as root, so the binary
# gets the one capability that lets it bind one — rather than the container
# getting root to spend on a socket.
RUN setcap cap_net_bind_service=+ep /usr/local/bin/rustycv-server

# `HOST` is the one that matters here: the binary binds loopback by default,
# which inside a container means nothing outside it can connect.
ENV RUSTYCV_WEB_DIST=/srv/web/dist \
    DATABASE_URL=sqlite:///data/rustycv.db \
    HOST=0.0.0.0 \
    PORT=80

# The CVs live here. Without a volume they go when the container does.
VOLUME /data
WORKDIR /data
USER rustycv
EXPOSE 80
CMD ["rustycv-server"]
