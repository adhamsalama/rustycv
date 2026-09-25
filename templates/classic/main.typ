// Classic — single column, generous whitespace, ATS-friendly.
//
// Reads the document from `/data.json`, which the renderer serves from memory.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let muted = luma(95)
#let base-size = theme.fontSizePt * 1pt
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
#set par(leading: theme.lineHeight * 0.65em, justify: false, spacing: theme.lineHeight * 0.75em)
#show link: set text(fill: accent)

#let section-heading(title) = {
  let styled = if theme.headingStyle == "caps" {
    text(weight: 700, size: base-size * 0.98, tracking: 0.08em, fill: accent, upper(title))
  } else {
    text(weight: 700, size: base-size * 1.12, fill: accent, title)
  }
  block(above: gap, below: 0.55em, {
    styled
    if theme.headingStyle == "underline" {
      v(0.25em, weak: true)
      line(length: 100%, stroke: 0.6pt + accent.lighten(35%))
    }
  })
}

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  // An employer heading sits a step above an entry title so the reader can tell
  // "who I worked for" from "what I did there" at a glance.
  company: body => text(weight: 700, size: base-size * 1.1, body),
  meta: body => text(fill: muted, size: base-size * 0.95, body),
  tight: 1.0,
)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, {
  text(size: base-size * 2.0, weight: 700, basics.fullName)
  if nonempty(basics.headline) {
    v(0.25em, weak: true)
    text(size: base-size * 1.15, fill: accent, weight: 500, basics.headline)
  }
})

#let bits = contact-bits(basics)
#if bits.len() > 0 {
  v(0.6em, weak: true)
  text(size: base-size * 0.95, bits.join(text(fill: muted)[ #sym.dot.c ]))
}

#if nonempty(basics.summary) {
  v(0.7em, weak: true)
  basics.summary
}

#render-sections(cv, style)
