// Modern — the accent colour does real work: a tinted name, ruled section
// headings, and a hairline under the header block.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let muted = luma(105)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.68em
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
#set par(leading: leading, justify: false, spacing: theme.lineHeight * 0.78em)
#show link: set text(fill: accent)

// Headings always carry a rule here — it is the template's defining move — so
// `headingStyle` only chooses between sentence case and small caps.
#let section-heading(title) = {
  let label = if theme.headingStyle == "caps" {
    text(weight: 700, size: base-size * 0.92, tracking: 0.12em, fill: accent, upper(title))
  } else {
    text(weight: 700, size: base-size * 1.1, tracking: 0.02em, fill: accent, title)
  }
  block(above: gap, below: 0.6em, {
    label
    v(0.3em, weak: true)
    line(length: 100%, stroke: 0.8pt + accent.lighten(55%))
  })
}

#let marker = bullet-marker(theme)

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  company: body => text(weight: 700, size: base-size * 1.12, fill: accent.darken(10%), body),
  meta: body => text(fill: muted, size: base-size * 0.92, body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.05,
)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, {
  text(size: base-size * 2.2, weight: 700, fill: accent, basics.fullName)
  if nonempty(basics.headline) {
    v(0.4em, weak: true)
    // h(0.8em)
    text(size: base-size * 1.3, weight: 500, tracking: 0.04em, basics.headline)
  }
})

#let bits = contact-bits(basics)
#if bits.len() > 0 {
  v(0.65em, weak: true)
  text(size: base-size * 0.92, fill: muted, bits.join(text(fill: accent.lighten(30%))[ | ]))
}

#v(0.6em, weak: true)
#line(length: 100%, stroke: 1.2pt + accent)

#if rich-nonempty(basics.summary) {
  v(0.8em, weak: true)
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
