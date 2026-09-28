// Editorial — a magazine page. The name is a large display serif set tight,
// each section opens with its number in the accent ("01 — Experience") over a
// hairline, and the summary is a pull quote: serif italic, a step larger,
// behind a hung opening quote mark. The body stays in the theme's face, so the contrast
// between a serif display and whatever text face is picked is the look.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let display = "Source Serif 4"
#let accent = rgb(theme.accent)
#let muted = luma(100)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.66em
#let gap = theme.sectionGapMm * 1mm

#set document(
  title: join-parts((basics.fullName, basics.headline), sep: " – "),
  author: basics.fullName,
)

#set page(
  paper: if theme.page == "letter" { "us-letter" } else { "a4" },
  margin: theme.marginMm * 1mm,
)

#set text(font: theme.fontFamily, size: base-size, lang: "en", fallback: true)
#set par(leading: leading, justify: false, spacing: theme.lineHeight * 0.75em)
#show link: set text(fill: accent)

#let section-number = counter("editorial-section")

// Hairline above, then the number and the title. The rule sits *above* the
// heading, the way a magazine opens a department; `underline` moves a second
// one below it as well.
#let section-heading(title) = {
  section-number.step()
  let caps = theme.headingStyle == "caps"
  block(above: gap, below: 0.6em, width: 100%, {
    line(length: 100%, stroke: 0.5pt + luma(160))
    v(0.45em, weak: true)
    context text(font: display, size: base-size * 0.95, weight: 600, fill: accent,
      numbering("01", section-number.get().first()))
    h(0.5em)
    if caps {
      text(size: base-size * 0.92, weight: 700, tracking: 0.16em, upper(title))
    } else {
      text(font: display, size: base-size * 1.3, weight: 600, [— ] + title)
    }
    if theme.headingStyle == "underline" {
      v(0.35em, weak: true)
      line(length: 100%, stroke: 0.5pt + accent.lighten(40%))
    }
  })
}

#let marker = bullet-marker(theme, dot: text(fill: accent)[•])

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  company: body => text(font: display, weight: 600, size: base-size * 1.18, body),
  meta: body => text(fill: muted, size: base-size * 0.9, style: "italic", body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.05,
)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, {
  text(font: display, size: base-size * 3.2, weight: 400, tracking: -0.02em, basics.fullName)
  if nonempty(basics.headline) {
    // Clears the display name's descenders, which are a size of their own.
    v(0.7em, weak: true)
    text(size: base-size * 0.95, weight: 700, tracking: 0.16em, fill: accent, upper(basics.headline))
  }
  let bits = contact-bits(basics)
  if bits.len() > 0 {
    v(0.55em, weak: true)
    text(size: base-size * 0.92, fill: muted, bits.join(h(0.5em) + text(fill: accent)[/] + h(0.5em)))
  }
})

#if rich-nonempty(basics.summary) {
  v(1.1em, weak: true)
  // The pull quote: an oversized opening mark hung in the indent, in the
  // accent. A mark rather than a bar down the side, so the quote's lines stay
  // separate lines of ink. Inset in em, so it grows with the type.
  block(inset: (left: 1.6em), {
    place(top + left, dx: -1.6em, dy: -0.25em,
      text(font: display, size: base-size * 3.2, fill: accent, weight: 600, "\u{201C}"))
    set text(font: display, style: "italic", size: base-size * 1.18)
    rich(basics.summary, gap: leading, marker: marker)
  })
}

#render-sections(cv, style)
