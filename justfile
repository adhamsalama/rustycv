# RustyCV dev tasks. `just` with no argument lists them.
default:
    @just --list

db := "sqlite://rustycv.db"

# The port the server listens on. `PORT` in the environment beats this default,
# and an argument beats both: `just serve-bundled 9000`.
default_port := env_var_or_default("PORT", "8080")

# Backend on :8080 and Vite on :5173, both watching.
dev:
    #!/usr/bin/env bash
    trap 'kill 0' EXIT
    DATABASE_URL="{{db}}" cargo run -p rustycv-server --bin rustycv-server &
    pnpm -C web dev &
    wait

# Backend only.
serve:
    DATABASE_URL="{{db}}" cargo run -p rustycv-server --bin rustycv-server

# Load fixtures/adham.json into the database.
seed:
    DATABASE_URL="{{db}}" cargo run -p rustycv-server --bin seed

# Build the frontend, then serve everything from the Rust binary.
serve-bundled port=default_port:
    pnpm -C web build
    DATABASE_URL="{{db}}" PORT="{{port}}" cargo run --release -p rustycv-server --bin rustycv-server

# Build the browser renderer into web/public/wasm/.
#
# Optional: without it the editor still works, the toggle reports that the
# module is not there, and every render goes to the server. `pnpm build` copies
# whatever is in public/ into dist/, so this has to run first to be shipped.
wasm:
    #!/usr/bin/env bash
    set -euo pipefail
    # The CLI has to match the crate exactly — a mismatch fails with a schema
    # error that says nothing about versions.
    wanted=$(awk '/^name = "wasm-bindgen"$/ {getline; gsub(/[",]/, "", $3); print $3; exit}' Cargo.lock)
    have=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
    if [ "$have" != "$wanted" ]; then
        echo "wasm-bindgen $wanted is needed; found '${have:-nothing}' on PATH." >&2
        echo "  rustup target add wasm32-unknown-unknown" >&2
        echo "  cargo install wasm-bindgen-cli --version $wanted --locked" >&2
        exit 1
    fi
    cargo build -p rustycv-wasm --target wasm32-unknown-unknown --profile wasm-release
    wasm-bindgen --target web --no-typescript --out-dir web/public/wasm \
        target/wasm32-unknown-unknown/wasm-release/rustycv_wasm.wasm
    # Shaves a few megabytes off. Nice to have, not required — the module is
    # correct either way.
    if command -v wasm-opt >/dev/null; then
        wasm-opt -Oz -o web/public/wasm/rustycv_wasm_bg.wasm web/public/wasm/rustycv_wasm_bg.wasm
    else
        echo "note: wasm-opt not on PATH — shipping the unoptimised module" >&2
    fi
    ls -lh web/public/wasm

test:
    cargo test --workspace
    pnpm -C web test
    pnpm -C web typecheck

lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --check

# Render every template from the fixture into target/render-out/.
render:
    cargo test -p rustycv-render --test render
    @echo "wrote target/render-out/"

# Start over with an empty database.
reset-db:
    rm -f rustycv.db rustycv.db-wal rustycv.db-shm
