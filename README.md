# RustyCV

A resume builder that stores **your CV as structured data, never as a rendered
PDF**.

The PDF is a pure function of `(document, template, theme)` and is recomputed on
every request. That one decision is what the whole design hangs off: switching
template, changing the accent colour, or duplicating a CV to tailor it for a
specific application costs nothing and loses nothing, because there is no
baked artifact to fall out of sync. Export gives you your data back, not a
picture of it.

|               |                                                                                                                                                                 |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Rendering** | [Typst](https://typst.app), linked as a Rust library and compiled from an in-memory `World` — no subprocess, no temp files, no filesystem access from templates |
| **Backend**   | Rust · axum · SQLite via sqlx                                                                                                                                   |
| **Frontend**  | React · Vite · TypeScript · TipTap for rich text                                                                                                                |
| **Speed**     | ~5 ms for a warm render of a full CV in release (~26 ms in a debug build) — which is what makes the live preview feel instant                                   |

Four built-in templates, including `flowcv` — a faithful reproduction of
FlowCV's default single-column layout, measured from a real published resume.

Whole sections **and individual entries** can be hidden rather than deleted, so
one CV can be tailored per application without losing anything. A section whose
entries are all hidden leaves no trace — not even its heading.

Description fields — summaries, entry notes, and work-experience highlights —
support **bold, italic, underline and links**. The formatting is stored as a
flat list of styled runs rather than HTML or Markdown, so nothing ever parses
untrusted markup on its way into a PDF; text that carries no marks is still
stored as a plain string, keeping exports readable.

Work Experience carries two switches of its own, honoured by every template
because they are document data rather than template decoration: whether an
entry reads _Job title – Employer_ or _Employer – Job title_, and whether
consecutive roles at one employer are grouped under a single heading
("group promotions").

---

## Prerequisites

| Tool        | Minimum               | Notes                                                                                                                                      |
| ----------- | --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| **Rust**    | 1.92                  | Required by Typst 0.15. Pinned in `Cargo.toml`, so an older toolchain fails with a clear message. Install via [rustup](https://rustup.rs). |
| **Node.js** | `^20.19` or `>=22.12` | Required by Vite 7. Note 20.0–20.18 and 21.x will _not_ work.                                                                              |
| **pnpm**    | 10+                   | Needed for the `onlyBuiltDependencies` setting in `pnpm-workspace.yaml`. `corepack enable pnpm`, or `npm i -g pnpm`.                       |

Nothing else. SQLite is compiled in (`libsqlite3-sys` bundles it), and fonts and
icons are embedded in the binary — there is no system dependency to install and
no `typst` CLI needed.

> **First build takes a few minutes.** The Typst dependency tree is large.
> Every build after that is incremental and fast. `target/` grows to several GB
> with test artifacts; `cargo clean` reclaims it.

---

## Install

```sh
git clone <this-repo> rustycv
cd rustycv

pnpm -C web install     # frontend dependencies
cargo build             # backend + Typst (slow the first time)
```

If pnpm refuses to run esbuild's install script, approve it once:

```sh
pnpm -C web approve-builds esbuild --yes
```

Optionally load the sample CV so there's something to look at:

```sh
cargo run -p rustycv-server --bin seed
```

---

## Run

### Development — two processes, both hot-reloading

This is what you want while working on the app. Vite serves the UI and proxies
`/api` to the Rust server.

```sh
# terminal 1 — API on :8080
cargo run -p rustycv-server

# terminal 2 — UI on :5173
pnpm -C web dev
```

Open **http://localhost:5173**.

> Use `localhost`, not the literal `127.0.0.1` — Vite binds `[::1]` (IPv6), so
> `http://127.0.0.1:5173` will refuse the connection while `localhost` works.

Frontend edits hot-reload. Backend edits need a restart — or use
`cargo watch -x 'run -p rustycv-server'` if you have `cargo-watch`.

### Single process — everything from the Rust binary

The server serves the built frontend itself when `web/dist` exists, with an
SPA fallback so `/cv/:id` survives a hard refresh.

```sh
pnpm -C web build
cargo run --release -p rustycv-server
```

Open **http://localhost:8080**. This is also how you'd deploy it: one binary
plus `web/dist`.

### Configuration

All optional.

| Variable           | Default                               | Meaning                                                        |
| ------------------ | ------------------------------------- | -------------------------------------------------------------- |
| `DATABASE_URL`     | `sqlite://rustycv.db`                 | SQLite file. Created automatically; migrations run at startup. |
| `PORT`             | `8080`                                | API/server port.                                               |
| `RUSTYCV_WEB_DIST` | `web/dist`                            | Where the built frontend lives.                                |
| `RUST_LOG`         | `rustycv_server=info,tower_http=warn` | Standard `tracing` filter.                                     |
| `RUSTYCV_API`      | `http://127.0.0.1:8080`               | Vite dev-server proxy target (frontend only).                  |

```sh
DATABASE_URL=sqlite:///tmp/scratch.db PORT=9000 cargo run -p rustycv-server
```

---

## Rebuild

Which command you need depends on what you touched — most confusion here comes
from templates and assets being **compiled into the binary**, not read from disk
at runtime.

| You changed                    | Do this                                                                                                                              |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ |
| `web/**`                       | Nothing in dev (Vite reloads). For the bundled server: `pnpm -C web build`                                                           |
| Rust source                    | `cargo build` (or restart `cargo run`)                                                                                               |
| `templates/**.typ`             | `cargo build -p rustycv-render` — they're `include_str!`'d, so a stale binary keeps the old template                                 |
| `assets/fonts`, `assets/icons` | Same: `cargo build -p rustycv-render`                                                                                                |
| `migrations/**`                | `cargo build`, then restart. New migrations apply on startup; SQLite has no down-migrations here, so `rm rustycv.db*` to start clean |
| `fixtures/adham.json`          | `cargo run -p rustycv-server --bin seed`                                                                                             |
| Dependencies                   | `cargo build` / `pnpm -C web install`                                                                                                |

Full clean rebuild:

```sh
cargo clean && rm -rf web/node_modules web/dist
pnpm -C web install && pnpm -C web build && cargo build --release
```

Start over with an empty database:

```sh
rm -f rustycv.db rustycv.db-wal rustycv.db-shm
```

### Optional: `just`

A `justfile` wraps all of the above. It is convenience only — every task is a
plain command you can run directly.

```sh
brew install just     # or: cargo install just
just                  # list tasks
just dev              # API + Vite together
just seed / test / lint / render / reset-db
```

---

## Tests

```sh
cargo test --workspace     # 59 tests
pnpm -C web test           # 23 tests, the rich-text conversions
pnpm -C web typecheck
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

What's actually covered: the document model round-trips and tolerates sparse and
unknown fields; every template renders the fixture, an empty document, and one
blank entry of every section kind; hostile theme values are clamped rather than
rejected; the API round-trips create → save → reload → export → import; and
`flowcv` is asserted to still fit the reference resume on a single page.

Two of the stronger ones render to pixels rather than eyeballing: hiding every
entry in a section is asserted to render _identically_ to deleting the section,
and resetting dragged-about spacing sliders is asserted to reproduce the
reference resume byte for byte.

To eyeball the output, render every template to PDF and PNG:

```sh
cargo test -p rustycv-render --test render
open target/render-out/
```

---

## Project layout

```
crates/rustycv-core      the CV document model — the thing we persist
crates/rustycv-render    Typst World, templates, fonts, PDF/PNG export
crates/rustycv-server    axum routes, sqlx storage, seed binary
templates/               .typ sources, embedded at build time
assets/fonts, /icons     embedded at build time
migrations/              sqlx migrations
fixtures/adham.json      the reference resume `flowcv` targets
web/                     React editor
```

Three crates rather than one so that editing a route doesn't recompile the
Typst dependency tree.

## Templates

| id        | look                                                                                            |
| --------- | ----------------------------------------------------------------------------------------------- |
| `flowcv`  | Reproduction of FlowCV's default: centred header, tinted section bands, roles behind a hairline |
| `classic` | Single column, generous whitespace, ATS-friendly _(default for new CVs)_                        |
| `modern`  | Accent-coloured ruled headings, bolder name                                                     |
| `compact` | Tighter type and spacing, for CVs spilling onto a second page                                   |

`classic`, `modern` and `compact` share their section-rendering logic in
`templates/common.typ` and differ only in page setup plus a `style` dict of
typographic hooks, so a fix to entry layout lands in all of them at once.
`flowcv` carries its own entry geometry because its 55/45 split, hairline and
heading band are specific enough that reusing the shared ones would distort
both.

Users pick a template and turn the knobs in `Theme` — accent, font, size,
margins, line height, section gap, page size, heading style. **They never
author Typst**, so no untrusted code reaches the compiler, and the `World`
exposes nothing but the template, the icons and the CV itself.

Each template declares the spacing it was designed around, served on
`/api/templates` as `metrics`, and the editor's **Reset spacing** control
restores text size, line height, margins and section gap to _that_ template's
values. They are per-template rather than app-wide on purpose: `flowcv`
reproduces a layout measured at 9pt with 10mm margins, so resetting it to the
generic 10pt/16mm would quietly break the reproduction.

## API

| Method               | Path                           | Purpose                                               |
| -------------------- | ------------------------------ | ----------------------------------------------------- |
| `GET`                | `/api/templates`, `/api/fonts` | what the theme picker offers                          |
| `GET` `POST`         | `/api/cvs`                     | list; create (`?from=<id>` duplicates)                |
| `GET` `PUT` `DELETE` | `/api/cvs/{id}`                | fetch / save / delete                                 |
| `POST`               | `/api/render`                  | body = document → `application/pdf`; the live preview |
| `GET`                | `/api/cvs/{id}/pdf`            | download, named after the person                      |
| `GET`                | `/api/cvs/{id}/export`         | the document as JSON                                  |
| `POST`               | `/api/cvs/import`              | JSON → new CV                                         |

Preview and download share one render path, so what you see is what you get.
A template that fails to compile returns **422** with structured Typst
diagnostics (file, line, message) rather than a 500 and a blank preview — the
editor overlays them on the last good render.

```sh
curl -X POST http://localhost:8080/api/render \
  -H 'content-type: application/json' \
  --data-binary @fixtures/adham.json -o cv.pdf
```

## Troubleshooting

**`cargo run` says it can't pick a binary** — the server package also ships the
`seed` binary. `default-run` resolves this, so plain `cargo run` works; if your
tooling bypasses it, be explicit:
`cargo run -p rustycv-server --bin rustycv-server`.

**Template edits don't show up** — templates are compiled in. Rebuild
(`cargo build -p rustycv-render`) and restart the server.

**`ERR_PNPM_IGNORED_BUILDS`** — `pnpm -C web approve-builds esbuild --yes`.

**Port already in use** — `PORT=9000 cargo run -p rustycv-server`, and set
`RUSTYCV_API=http://127.0.0.1:9000` for the Vite proxy.

**`http://127.0.0.1:5173` refuses the connection** — Vite listens on IPv6
`[::1]`. Use `http://localhost:5173`.

**`/` returns 404 but `/api/...` works** — `web/dist` didn't exist when the
server started, so no static handler was mounted. Run `pnpm -C web build`, then
restart the server.

## Third-party assets

Fonts (Inter, IBM Plex Sans, Source Sans 3, Source Serif 4 — all SIL OFL) and
icons (Font Awesome Free 6, CC BY 4.0) are embedded in the binary. See
[NOTICE.md](NOTICE.md); licence texts are in `assets/licenses/`.

The `flowcv` template is an independent Typst reimplementation of a layout, not
a copy: no FlowCV code, stylesheet or asset is redistributed here.
