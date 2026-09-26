// Engineer — the single-column technical resume: no columns, no graphics, no
// icons, nothing a keyword parser has to guess at.
//
// What it changes over `classic` is emphasis rather than decoration. The
// headline and contact details share one line so the header costs two lines
// instead of four; section headings are set small and letter-spaced with a rule
// running from the label out to the right margin; and the rhythm is a notch
// tighter, because the CVs this is for are bullet-heavy. It buys back space
// from the layout rather than from the type, which is what separates it from
// `compact`.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let muted = luma(100)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.62em
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
#set par(leading: leading, justify: false, spacing: theme.lineHeight * 0.72em)
#show link: set text(fill: accent)

// The defining move: the heading label, then a hairline carrying on to the
// right margin. `underline` gets the plainer full-width rule underneath so the
// control still chooses between three visibly different headings.
#let section-heading(title) = {
  let caps = theme.headingStyle == "caps"
  let label = text(
    weight: 700,
    size: base-size * (if caps { 0.95 } else { 1.08 }),
    tracking: if caps { 0.1em } else { 0em },
    fill: accent,
    if caps { upper(title) } else { title },
  )
  block(above: gap, below: 0.5em, width: 100%, if theme.headingStyle == "underline" {
    label
    v(0.22em, weak: true)
    line(length: 100%, stroke: 0.7pt + accent.lighten(30%))
  } else {
    grid(
      columns: (auto, 1fr),
      column-gutter: 0.7em,
      align: horizon,
      label,
      line(length: 100%, stroke: 0.7pt + accent.lighten(45%)),
    )
  })
}

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  // An employer heading sits a step above an entry title so the reader can tell
  // "who I worked for" from "what I did there" at a glance.
  company: body => text(weight: 700, size: base-size * 1.1, body),
  meta: body => text(fill: muted, size: base-size * 0.93, body),
  leading: leading,
  line-height: theme.lineHeight,
  // Denser than classic, nowhere near compact: this template keeps full-size
  // type and buys its page back from the vertical rhythm alone.
  tight: 0.88,
)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, text(size: base-size * 1.95, weight: 700, tracking: -0.01em, basics.fullName))

// Headline left, contact details right-aligned on the same line. The contact
// side takes the free width rather than `auto`, so a long list wraps within its
// own column instead of pushing the headline off the page.
#let bits = contact-bits(basics)
#if nonempty(basics.headline) or bits.len() > 0 {
  v(0.35em, weak: true)
  grid(
    columns: (auto, 1fr),
    column-gutter: 1.2em,
    align: horizon,
    if nonempty(basics.headline) {
      text(size: base-size * 1.1, weight: 600, fill: accent, basics.headline)
    },
    align(right, if bits.len() > 0 {
      text(size: base-size * 0.93, fill: muted, bits.join(text(fill: muted)[ #sym.dot.c ]))
    }),
  )
}

#if rich-nonempty(basics.summary) {
  v(0.7em, weak: true)
  rich(basics.summary, gap: leading)
}

#render-sections(cv, style)
