# syntax=docker/dockerfile:1

# The browser renderer: the same Rust render path compiled to wasm, so the
# editor can build a PDF without a round trip. First of the three stages,
# because `pnpm build` ships whatever is sitting in web/public/ and this is
# what puts it there.
FROM rust:1.95-bookworm AS wasm
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY templates templates
COPY assets assets
# The wasm-bindgen CLI has to match the `wasm-bindgen` crate exactly, so the
# version is read out of the lockfile rather than pinned here — otherwise a
# `cargo update` leaves the two disagreeing, and a mismatch fails with a schema
# error that mentions no versions at all.
RUN set -eux; \
    version="$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/[",]/, "", $3); print $3; exit }' Cargo.lock)"; \
    case "$(uname -m)" in \
      x86_64)  arch=x86_64 ;; \
      aarch64) arch=aarch64 ;; \
      *) echo "no wasm-bindgen release for $(uname -m)" >&2; exit 1 ;; \
    esac; \
    release="wasm-bindgen-${version}-${arch}-unknown-linux-musl"; \
    curl -sSfL "https://github.com/wasm-bindgen/wasm-bindgen/releases/download/${version}/${release}.tar.gz" \
      | tar xz --strip-components=1 -C /usr/local/bin "${release}/wasm-bindgen"; \
    rustup target add wasm32-unknown-unknown; \
    cargo build -p rustycv-wasm --target wasm32-unknown-unknown --profile wasm-release; \
    wasm-bindgen --target web --no-typescript --out-dir /wasm \
      target/wasm32-unknown-unknown/wasm-release/rustycv_wasm.wasm

# The editor. Built separately from the server: it changes on its own
# schedule, and nothing in the Rust build depends on it.
FROM node:22-bookworm-slim AS web
WORKDIR /web
RUN corepack enable && corepack prepare pnpm@12.3.4 --activate
# Dependencies before sources, so editing a component does not reinstall them.
# `pnpm-workspace.yaml` comes too: it is what allows esbuild's postinstall to
# run, and pnpm fails the install rather than silently skipping it.
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml web/.npmrc ./
RUN pnpm install --frozen-lockfile
COPY web/ ./
# Into public/, which Vite copies verbatim into dist/. A build without this
# still works — the editor finds no module and every render goes to the server.
COPY --from=wasm /wasm ./public/wasm
RUN pnpm build
# Compress what the server will serve, once, here. `ServeDir` is configured to
# send these (`precompressed_br`/`precompressed_gzip`) rather than compress per
# request — which for a thirty-megabyte wasm module would cost the server more
# CPU than the renders that module exists to take off it. `just wasm` does the
# same for a local build, which only covers the module itself.
#
# `.mjs` is on the list because pdf.js's worker is one, and at 1.2MB it is the
# largest thing the editor loads after the wasm module — an extension missing
# from here is a file served raw, which shows up nowhere except as a slow
# first preview.
RUN apt-get update \
 && apt-get install -y --no-install-recommends brotli \
 && rm -rf /var/lib/apt/lists/* \
 && find dist -type f \( -name '*.wasm' -o -name '*.js' -o -name '*.mjs' \
      -o -name '*.css' -o -name '*.html' -o -name '*.json' -o -name '*.svg' \) \
      -exec gzip -9 -k -f {} \; -exec brotli -q 11 -f {} \;

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
