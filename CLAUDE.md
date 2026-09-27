# CLAUDE.md

Working notes for this repo. `README.md` covers what RustyCV is and how to run
it — this file is the things that are easy to get wrong.

## The one invariant

**A CV is data. The PDF is a pure function of `(document, template, theme)`,
recomputed on demand and never stored.** Everything else follows from this.
If a change would cache, persist or hand-edit a rendered PDF, it is wrong.

Two corollaries that are load-bearing:

- **Preview and download go through one render path** (`render_pdf`). The
  preview is exactly the output. Do not add a second, faster, approximate one.
  Rendering in the browser is not one: it is that same function compiled to
  wasm, and the fixture comes out to the same SHA-256 through both. Neither is
  `PdfDocument`: pdf.js *rasterises* the finished bytes for the screen, the
  way the browser's viewer did before it, and the download hands over the same
  `Blob` it was given. Drawing a page is not producing one.
- **The Typst `World` serves only memory** — the template, the icons, the CV.
  No filesystem, no packages. That is what makes running templates safe, so
  don't add a loader that reaches outside `CvWorld`.

## Layout

```
crates/rustycv-core     the document model. Pure serde, no heavy deps.
crates/rustycv-render   Typst World, templates, fonts, PDF/PNG export
crates/rustycv-server   axum routes, sqlx storage, accounts, seed binary
crates/rustycv-wasm     the same renderer, compiled for the browser
templates/              .typ sources, embedded with include_str!
assets/fonts, /icons    embedded with include_bytes!
fixtures/adham.json     the reference resume `flowcv` reproduces
web/                    React editor
```

Three crates so that editing a route does not recompile the Typst tree. Keep it
that way: don't move rendering into the server crate. The fourth is a shim, not
a peer — see *The browser renderer*.

**Templates and assets are compiled in.** Editing a `.typ` file and restarting
the server is not enough — `cargo build -p rustycv-render` first, or you will
debug a stale template.

## Commands

```sh
cargo run -p rustycv-server         # API + built UI on :8080
pnpm -C web dev                     # UI on :5173 (use localhost, Vite binds ::1)
cargo run -p rustycv-server --bin seed

cargo test --workspace              # 128 tests
pnpm -C web test                    # 64, the rich-text conversions and the render fallback
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

cargo test -p rustycv-render --test render   # writes target/render-out/*.pdf|png

just wasm                           # the browser renderer → web/public/wasm/
```

`just` wraps these. `just serve-bundled [port]` takes an optional port,
defaulting to `PORT` from the environment and then to 8080; the server itself
reads `PORT` and **falls back to 8080 without complaint** if it cannot parse it,
so a typo'd port looks like the flag being ignored.

## Gotchas that have each cost real time

**Typst resolves the space between two blocks as the *maximum* of the first's
`below` and the second's `above`.** So a spacing bug usually needs *both* sides
fixed, and reverting one to reproduce it will appear to work fine. When
verifying a spacing regression test, check out the previous commit's whole
template rather than reverting one line.

**`#[serde(rename_all)]` on an enum renames its *variants*, not the fields
inside them.** Variant fields need `rename_all_fields`. This shipped
`group_promotions` as snake_case while the templates read `groupPromotions`,
and the switch silently did nothing.
`every_serialized_key_is_camel_case` now guards the whole document.

**Anything a theme slider controls must scale.** Three separate bugs came from
gaps hardcoded in `em` while the leading around them scaled with `lineHeight`.
Inside an entry, one line step is `style.leading` (or `leading` in flowcv);
breaks *between* entries multiply by `style.line-height` too. Absolute `pt`/`mm`
is for strokes, radii and page geometry only — never for text spacing.

**A gap of exactly one line step is invisible.** The step from one line to the
next already *is* the leading, so a block spaced by `style.leading` renders
identically to a wrapped line — which is what the first attempt at the paragraph
break in a description did. Blocks inside a description sit *two* steps
apart; list items inside one list sit one, matching an entry's highlights.
`a_paragraph_break_is_a_wider_step_than_a_line_break` pins it and failed on
exactly that bug.

**Typst breaks the line on a newline inside a *text value*** — unlike a newline
written in markup, which is ordinary whitespace. That is what makes a soft break
(stored as `\n` in a run) work without an explicit `linebreak()`.

**`typst` 0.15 API notes**: `FileId::new` takes a `RootedPath`, not
`(Option<PackageSpec>, VirtualPath)`. `PagedDocument` lives in `typst-layout`,
not re-exported from `typst`. Diagnostic spans are `DiagSpan`, and hints are
`Spanned<EcoString>` — use `.v`.

## Templates

`classic`, `modern`, `engineer`, `banner` and `compact` share all their section
rendering in `templates/common.typ` and differ only in page setup plus a `style`
dict of typographic hooks. A fix to entry layout belongs there, once — a new
template that needs different *entry geometry* is the rare case, not the norm.

`banner` is the only one that sets text **on** the accent, so it derives its ink
from the accent's brightness rather than assuming a dark one; anything it draws
on the paper instead uses the accent darkened back to a readable weight.
`the_banner_band_picks_ink_that_survives_the_accent` pins both halves.

Rich text has two entry points. `rich(value, gap:, marker:)` returns *block*
content — paragraphs and lists; `rich-inline(value)` returns inline content, for
a list item's body or a project's one-line description. A value that is a single
paragraph comes back from `rich` as bare inline content on purpose: the weak
spacing templates set around a summary then collapses exactly as it did before
blocks existed, which is what keeps every pre-blocks CV rendering unchanged.
`rich-inlineable(value)` is how a project decides between the two.

`flowcv` carries its own entry geometry on purpose — its 55/45 title/date
split, employer hairline and heading band are specific enough that reusing the
shared ones would distort both. It **reproduces a real published resume**; its
numbers were measured from that page's rendered markup and the comments record
FlowCV's own values. Don't "tidy" them, and check `fixtures/adham.json` still
renders on one page after touching it.

Each template declares its own `Metrics` (the spacing the reset control
restores). `flowcv` is 9pt/10mm because that is what it was measured at.

## Testing

Rendering bugs are invisible in a diff, so **the render tests assert on
pixels**. `render_pngs` plus the `image` crate; helpers in
`crates/rustycv-render/tests/render.rs`:

- `hairline_segments` — counts contiguous runs in the employer rule's column
- `text_line_starts` — where each line of text begins, for measuring rhythm
- `content_height` — first to last inked row

Prefer line *positions* over total ink height: descenders shift the latter by
several pixels and will send you chasing a layout bug that isn't there.

**A spacing test you have not seen fail is not a test.** Two in this repo
originally passed on the very bug they were written for — one with a tolerance
too loose, one measuring the wrong thing. Break the template, watch it fail,
then fix it back.

Two audit tests exist to catch whole classes of this:
`every_theme_control_does_something_on_every_template` (dead controls) and
`spacing_scales_with_type_size` (absolute units).

## The browser renderer

**`RUSTYCV_RENDER` is an operator's flag, not a user's setting.** `browser`
(the default) or `server`, read once in `main.rs`, served to the editor over
`/api/config`, and not overridable from the client — there is no stored
preference and nothing in the UI. An unrecognised value **refuses to start**
rather than defaulting; that is deliberate, and it is the `PORT` trap above
written the other way round, because a mode that was quietly ignored looks
exactly like one that was applied. `build_app` takes it as an argument rather
than reading the environment itself, so a test's router does not depend on the
shell that ran it.

The editor **waits for `/api/config`** before its first render (`Editor.tsx`
gates on it alongside the document). Guessing and correcting would mean one
render on the machine the operator ruled out.

`crates/rustycv-wasm` is a shim — a `render_json` that parses and calls
`rustycv_render::render_pdf`, and three `#[wasm_bindgen]` lines over it. It
must stay that. The moment it grows a decision of its own it becomes the second
render path the invariant forbids;
`a_browser_render_is_the_same_bytes_as_a_server_render` is what notices.

**The module is optional at every level.** It is gitignored, the editor builds
and runs without one, and the render path falls back to the server rather than
failing — which is what makes `browser` safe as the default on an instance
nobody built one for. Don't make anything depend on its being there.

**Templates and fonts are compiled into it too.** So the rule about rebuilding
`rustycv-render` after touching a `.typ` file or `assets/` applies twice: `just
wasm` as well, or the browser keeps rendering the old template while the server
renders the new one — which looks like the flag changing the output, the one
thing it must never do.

`just wasm` writes into `web/public/wasm/`, which Vite copies verbatim into
`dist/`, so it has to run **before** `pnpm -C web build`. It is not a
dependency of `dev` or `serve-bundled` — the wasm toolchain stays optional —
but `_wasm-note` runs ahead of both and says so when the module is missing or
older than `templates/`, `assets/` or the renderer's source. Nothing imports it at
build time: `renderWorker.ts` fetches `/wasm/rustycv_wasm.js` at runtime behind
`/* @vite-ignore */`, which is what keeps a missing module a runtime fallback
instead of a build error.

**The module is compressed at build time, never per request.** `just wasm`
writes `.wasm.br` and `.wasm.gz` beside it and `ServeDir` is built with
`precompressed_br`/`precompressed_gzip`, so the server sends a file rather than
running brotli over 30MB on every cold load — which would cost it more than the
renders the module exists to take off it. A `CompressionLayer` is the wrong
tool here for exactly that reason. Numbers: 31.7MB raw, 11.7MB gzip, 7.9MB
brotli, and the content type survives (`application/wasm`, which
`instantiateStreaming` requires).

Which means **the compressed copies can go stale**. `ServeDir` prefers them, so
an out-of-date `.br` is served to every browser that accepts one while a bare
`curl` gets the new module — a divergence with no symptom until someone
compares the two. `just wasm` always rewrites both, and `_wasm-note` checks
their mtimes.

The wasm-bindgen **CLI has to match the `wasm-bindgen` crate exactly**. A
mismatch fails with a schema error naming no versions, so `just wasm` and the
Dockerfile both read the wanted version out of `Cargo.lock` and refuse first.

`uuid` needs its `js` feature on `wasm32-unknown-unknown` — a document with an
id-less entry mints one while deserializing, and there is no OS to ask. It is a
target-specific dependency of `rustycv-wasm` so the server build is untouched.
`typst-render` goes the other way: `raster` is off in the workspace default and
the render crate's own tests turn it back on, so the module carries the PDF
exporter alone.

**A failed local render is two different things and they want opposite
answers.** The crate throws its failure body as JSON; anything else — a trap, a
panic message — is the module falling over. A JSON body means the *document*
does not compile, so it is shown, because the server would fail the same way
after a round trip. Everything else falls back to the server, and a panic also
discards the worker, since wasm memory after one is not something to render on
top of. `classifyLocalFailure` in `web/src/renderer.ts` is that rule and
`renderer.test.ts` pins it, including the case that nearly broke it: a panic
message that happens to parse as JSON is still a panic.

A *load* failure switches browser rendering off for the life of the page; a
panic costs only the instance. The difference is whether the next attempt would
differ. Both fall back **towards** the server and never away from it — the flag
is a ceiling on where work may happen, not a hint.

The preview says nothing when a render happened where it was configured to.
The "Rendered on the server" badge appears only on a fallback, because that is
the only visible sign a module is missing or broken.

**A downloaded PDF's name is written twice.** The server puts it in
`Content-Disposition`; a local render has no response to hang a header on, so
`pdfFilename` in `web/src/renderer.ts` rebuilds it. It mirrors `slug` in
`routes.rs`, and `the_download_name_matches_the_editors` and the matching
`describe` in `renderer.test.ts` are the same case list on purpose.

**Share links always render on the server**, whatever the flag says
(`download_published_pdf`, which also caches against `updated_at`). A visitor
is sent the PDF and never the document, so there is nothing in the page to
render from — and rendering locally would trade a cache hit for a 30 MB
download and a cold compile.

## The preview

The editor draws the PDF itself, on one `<canvas>` per page (`PdfDocument`),
rather than handing the bytes to an `<iframe>`.

**`#toolbar=0` is a Chromium parameter.** Firefox's pdf.js ignores it and so
does Safari, so an iframe gave a Firefox user a search box, a page spinner, a
zoom menu and five annotation tools wrapped around a document that is thrown
away on the next keystroke. That is the whole reason for the change; the three
things it buys are consequences. The page count is one. A scroll position that
survives an edit is the second: the pages are swapped in a single
`replaceChildren`, so the container never collapses, where changing an
iframe's `src` sent every edit back to the top. A shadow under the paper is
the third.

**The rasterising happens in `pdfWorker.ts`, not here.** pdf.js splits its work
in two — the parser runs in a worker of its own, but painting the operator
list onto a canvas happens wherever the *document proxy* lives. Hold that proxy
on the main thread and every re-render rasterises an A4 page on the thread
answering the keyboard, which an iframe's viewer (in Chrome, another process
entirely) was not doing. So the proxy lives in a worker, paints onto
`OffscreenCanvas`, and sends back an `ImageBitmap` per page; a canvas with a
`bitmaprenderer` context *adopts* one rather than drawing it.

Three things that worker has to get right, each of which is a `document`
reference away from breaking:

- pdf.js makes canvases of its own for soft masks, patterns and transparency
  groups, and its default factory reaches for `document`. It gets an
  `OffscreenCanvas` one, duck-typed because `BaseCanvasFactory` is not public.
- `disableFontFace: true`, because there is no DOM to install an `@font-face`
  into — glyphs are drawn from the embedded font programs instead. Typst
  embeds every font it uses, so nothing is lost.
- pdf.js reads `window.location` when it spawns its parser, so the parser is
  constructed here with an explicit `port` and handed over. One per worker:
  `getDocument` makes a fresh `PDFWorker` when it is not given one and
  `task.destroy()` terminates it, which would mean spawning and killing a
  worker over a 1.2MB script on every pause in typing.

**There is no middle tier that paints on the main thread**, and that is
deliberate: a worker is its own module graph, so a fallback copy of pdf.js is
a *second* 131kB gzipped in the bundle for a path nothing reaches — every
browser new enough for the `light-dark()` this stylesheet is built on (Safari
17.5) has had `OffscreenCanvas` since 16.4. A worker that fails goes straight
to the iframe, toolbar and all, and stops being retried.

**The worker is a `.mjs`.** The Dockerfile's precompression step matches
extensions by name, so that one had to be added to the list; at 1.2MB it is
the largest thing the editor loads after the wasm module, and an extension
missing from that `find` is a file served raw with no symptom but a slow first
preview.

`@napi-rs/canvas` is in `ignoredOptionalDependencies` — pdfjs-dist wants it to
rasterise in *Node*, which nothing here does, and left alone it puts a native
binary in `node_modules` and the whole platform matrix in the lockfile.

The *share* page still uses `<embed>` and the browser's own viewer on purpose.
A visitor holding a link wants print and download, and should not fetch pdf.js
to read one page once.

## Editor appearance

The UI's light/dark is separate from the CV's own theme and never reaches the
render path — a CV is a printed page and its PDF is white either way.

Colour is **two layers**. A *palette* sets eleven base tokens and nothing else;
`:root` carries the default (Everforest) and each `[data-palette='…']` block
replaces the same eleven. Everything else — `--accent-soft`, `--hover`,
`--focus-ring`, `--warn-bg`, the shadows — is `color-mix`'d from those in a
shared block, so a new palette has nowhere to forget a hover state. Every base
token is a `light-dark()` pair: a palette is chosen independently of light/dark
and has to work in both.

**The palette blocks are not anchored to `:root`.** A bare
`[data-palette='gruvbox']` selector means any element wearing the attribute
resolves that palette's tokens, which is how the picker's swatches paint
themselves in a palette that is not the one currently on — without a second,
hand-maintained copy of the colours in TSX. Note that `--accent` is *derived*
and therefore inherited from the root, so a swatch reads `--accent-base`.

Three preferences, three keys, three apply functions (`theme.ts`): appearance
(`system` writes no attribute), palette (the default writes no attribute), and
a custom accent. The accent is stored as the **derived** `light-dark()` pair
rather than the raw hex, so the inline script in `index.html` only applies it —
`deriveAccent` is the only place the arithmetic lives. That script has to stay
in step with all three apply functions.

A single picked colour cannot serve both schemes, so `deriveAccent` walks it
towards black until it reads on a pale page and towards white until it reads on
a deep one. The bounds (≤0.36 and ≥0.45 relative luminance) are read off the
built-in palettes rather than invented, and they straddle the ink threshold
from both sides, which is why the ink pair is always white-on-light and
dark-on-dark. Nothing comes back unchanged from both halves; that is the point,
not a bug.

**A literal colour outside the token block is a colour that only works in one
scheme**, and `names no scheme-specific colour outside the tokens` in
`theme.test.ts` fails on one. Its allowlist is where a genuine exception is
justified (the CV's paper, overlay scrims). Two siblings guard the rest: `each
define every base token` catches a half-written palette — which does not fail
loudly, it silently inherits one Everforest border into a Gruvbox editor — and
`are the same set the picker offers` catches a palette that exists in one place
and not the other.

**That audit was vacuous for its whole life until 2026-09-27.** `import CSS
from './styles.css?raw'` returns an *empty string* under vitest unless `css` is
on, and an empty string breaks nothing downstream: `''.indexOf(x)` is -1,
`slice(-1)` is one character, and one character contains no colours. `test: {
css: true }` in `vite.config.ts` is the fix and `has a stylesheet to audit at
all` is the guard. Anything that asserts on a file's *contents* wants a
non-empty check next to it.

`system` is the default and sets no attribute at all, so plain CSS follows the
OS; the toggle only writes `data-appearance` for an explicit choice.

The UI's own icons are `components/Icon.tsx` — one stroked 24-unit grid, drawn
in `currentColor`. They are not `assets/icons`, which are Font Awesome glyphs
compiled into the renderer and drawn inside a CV. Don't reach for a Unicode
glyph: `☰ ⠿ ⤢ ✕` render as whatever the platform has for that code point and
read as fallback characters rather than controls, which is what they replaced.

## Wire format

camelCase throughout. Rich text is a flat list of *blocks* — paragraph, bullet
or numbered — each a list of styled runs, never HTML or Markdown, so nothing
parses untrusted markup on the way into a PDF. A list is not a node: consecutive
items of one kind render as one list, which is what keeps the format flat.

Three wire forms, and a value is written in the smallest that fits: a bare string
(one unstyled paragraph), an array of runs (one styled paragraph), an array of
blocks. All three must render identically, and the two array forms are told apart
by *order* — blocks are tried first, and a run carries neither `kind` nor `runs`.
A soft line break lives inside a run as a `\n`, so "two lines, no marks" stays a
bare string. `rich.rs` and `web/src/rich.ts` implement the same three forms and
have to stay in step.

An experience entry's `bullets` is one of these values like any other
description — it used to be a `Vec<RichText>`, one value per bullet, and
`rich::highlights` still reads that shape into bullet blocks. Don't "simplify"
that deserializer away: it is the only thing keeping existing CVs' highlights
from collapsing into one run-on paragraph. The two shapes are distinguishable
because the old one is an array of *values*, and a string or a run array is
never a valid element of the new one.

New fields need `#[serde(default)]` so older documents keep loading. Every
render path must survive an empty document and a blank entry of every section
kind — there are tests for both.

## Accounts

Email plus an Argon2id hash, and an opaque session token in an HttpOnly cookie
(`auth.rs`). No verification, no *reset*, no OAuth — each wants a mail sender
this app does not have. Changing a password is different: you are signed in
already, so it needs no mail, and it evicts every session but the one asking.

**A wrong current password answers 400, not 401.** The editor reads any 401 as
"the session is gone" and drops to the landing page, so 401 there would throw
someone out of the app over a typo in a form field. `shouldRecheckSession` in
`web/src/session.ts` is the other half of that rule.

**Every row belongs to an account, and the scope lives in the query.** `db.rs`
and `jobs.rs` take a `user_id` argument and put `WHERE user_id = ?` on the
lookup itself rather than checking ownership around it, so a query that forgets
it is one account reading another's CVs rather than a missing guard somewhere
else. Somebody else's id is a **404**, not a 403: whether an id exists is not a
stranger's business. `one_account_cannot_reach_anothers_cv` and
`..._board` pin both halves, and they were watched failing against the
unscoped queries.

`user_id` is nullable because rows written before accounts existed have no
owner to name. **The first account created on such a database adopts them** —
without that, an instance that had been in use comes back looking empty, which
is indistinguishable from having lost the CVs.

The caps are 10 CVs and 10 applications per account, enforced *inside* the
`INSERT` (`INSERT ... SELECT ... WHERE (SELECT COUNT(*) ...) < ?`) so that
counting and writing cannot race. A refusal is **409**, because nothing is
forbidden — deleting one makes the identical request succeed.

Rate limiting is 1000 requests an hour per key, fixed window, in memory
(`ratelimit.rs`). The key is the account when there is one and the peer address
otherwise, which is why `resolve_session` runs *before* `rate_limit` and
`require_auth` runs *after* it: signed-in callers are limited per account rather
than per network, and an unauthenticated flood is still counted rather than
being rejected for free. **`X-Forwarded-For` is deliberately not read** — a
header the client writes is a limit the client opts out of. `main.rs` serves
with `into_make_service_with_connect_info`, without which every anonymous
caller shares one bucket.

The limit is a field on `RateLimiter`, not a read of the constant, so a test can
build one it can reach the end of. Argon2 is pinned to `opt-level = 3` in dev
builds (root `Cargo.toml`) — unoptimized it costs seconds per hash and would
dominate the suite, and tuning the cost down for the tests' sake is the wrong
fix.

## Scope

Accounts, but no TLS, and `CorsLayer::permissive()` still in debug builds. The
cookie crosses the wire in the clear, so the binary still binds `127.0.0.1`
unless `HOST` says otherwise — the Docker image overrides it to `0.0.0.0`
because a container that binds loopback is unreachable, so anything publishing
that port wants a TLS terminator in front, and `Secure` on the cookie with it.

Users never author Typst — they pick a template and turn theme knobs — so don't
add a raw-Typst escape hatch without sandboxing the compile.
