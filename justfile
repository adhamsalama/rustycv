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

test:
    cargo test --workspace
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
