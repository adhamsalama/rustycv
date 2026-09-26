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
- **The Typst `World` serves only memory** — the template, the icons, the CV.
  No filesystem, no packages. That is what makes running templates safe, so
  don't add a loader that reaches outside `CvWorld`.

## Layout

```
crates/rustycv-core     the document model. Pure serde, no heavy deps.
crates/rustycv-render   Typst World, templates, fonts, PDF/PNG export
crates/rustycv-server   axum routes, sqlx storage, seed binary
templates/              .typ sources, embedded with include_str!
assets/fonts, /icons    embedded with include_bytes!
fixtures/adham.json     the reference resume `flowcv` reproduces
web/                    React editor
```

Three crates so that editing a route does not recompile the Typst tree. Keep it
that way: don't move rendering into the server crate.

**Templates and assets are compiled in.** Editing a `.typ` file and restarting
the server is not enough — `cargo build -p rustycv-render` first, or you will
debug a stale template.

## Commands

```sh
cargo run -p rustycv-server         # API + built UI on :8080
pnpm -C web dev                     # UI on :5173 (use localhost, Vite binds ::1)
cargo run -p rustycv-server --bin seed

cargo test --workspace              # 83 tests
pnpm -C web test                    # 55, the rich-text conversions
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

cargo test -p rustycv-render --test render   # writes target/render-out/*.pdf|png
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

## Editor appearance

The UI's light/dark is separate from the CV's own theme and never reaches the
render path — a CV is a printed page and its PDF is white either way.

**Every editor colour is a `light-dark()` token in `:root`.** A literal colour
anywhere else in `styles.css` is a colour that only works in one scheme, which
is invisible until someone switches over — `names no scheme-specific colour
outside the tokens` in `theme.test.ts` fails on one, and its allowlist is the
place to justify a genuine exception (the CV's paper, overlay scrims).

`system` is the default and sets no attribute at all, so plain CSS follows the
OS; the toggle only writes `data-appearance` for an explicit choice. The inline
script in `web/index.html` applies the stored one before first paint and has to
stay in step with `applyAppearance`.

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

## Scope

No auth: single implicit local user, and `CorsLayer::permissive()` in debug
builds. Fine on localhost, not safe to expose. Users never author Typst — they
pick a template and turn theme knobs — so don't add a raw-Typst escape hatch
without sandboxing the compile.
