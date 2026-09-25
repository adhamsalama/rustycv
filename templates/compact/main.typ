// Compact — same content, less paper.
//
// Everything is dialled down rather than removed: smaller type, tighter
// rhythm, a single-line header. Useful when a CV is spilling a few lines onto a
// second page.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let muted = luma(100)
// Compact reads a notch below the requested size; the theme slider still moves
// it, this just shifts the whole range down.
#let base-size = theme.fontSizePt * 0.94pt
#let gap = theme.sectionGapMm * 0.7mm

#set document(
  title: join-parts((basics.fullName, basics.headline), sep: " – "),
  author: basics.fullName,
)

#set page(
  paper: if theme.page == "letter" { "us-letter" } else { "a4" },
  margin: theme.marginMm * 0.85mm,
)

#set text(font: theme.fontFamily, size: base-size, lang: "en", fallback: true)
#set par(leading: theme.lineHeight * 0.58em, justify: false, spacing: theme.lineHeight * 0.62em)
#show link: set text(fill: accent)

#let section-heading(title) = {
  let label = if theme.headingStyle == "caps" {
    text(weight: 700, size: base-size * 0.9, tracking: 0.1em, fill: accent, upper(title))
  } else {
    text(weight: 700, size: base-size * 1.05, fill: accent, title)
  }
  block(above: gap, below: 0.35em, {
    label
    if theme.headingStyle == "underline" {
      v(0.18em, weak: true)
      line(length: 100%, stroke: 0.5pt + accent.lighten(40%))
    }
  })
}

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, body),
  company: body => text(weight: 700, size: base-size * 1.05, body),
  meta: body => text(fill: muted, size: base-size * 0.93, body),
  // The whole point of this template: two-thirds of the usual vertical rhythm.
  tight: 0.62,
)

// ------------------------------------------------- header, on as few lines as possible

#block(spacing: 0pt, {
  text(size: base-size * 1.6, weight: 700, basics.fullName)
  if nonempty(basics.headline) {
    text(size: base-size * 1.6, weight: 700, fill: muted, "  ·  ")
    text(size: base-size * 1.05, weight: 500, fill: accent, basics.headline)
  }
})

#let bits = contact-bits(basics)
#if bits.len() > 0 {
  v(0.3em, weak: true)
  text(size: base-size * 0.92, bits.join(text(fill: muted)[ #sym.dot.c ]))
}

#if nonempty(basics.summary) {
  v(0.4em, weak: true)
  basics.summary
}

#render-sections(cv, style)
