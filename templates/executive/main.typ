// Executive — generous and understated. A wide-tracked capital name over a
// hairline, headings in a darkened accent and nowhere else, and the summary set
// a step larger as a lead paragraph. The whitespace is the design: it is the
// template for a CV that has nothing to prove by filling the page.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

// Headings in a vivid accent read as marketing; knocked back towards black they
// read as a letterhead. Links share it so the page carries one colour.
#let accent = rgb(theme.accent).darken(18%)
#let muted = luma(105)
#let base-size = theme.fontSizePt * 1pt
// A touch more air than classic — this template is for fewer, longer lines.
#let leading = theme.lineHeight * 0.72em
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
#set par(leading: leading, justify: false, spacing: theme.lineHeight * 0.8em)
#show link: set text(fill: accent)

#let section-heading(title) = {
  let caps = theme.headingStyle == "caps"
  let label = if caps {
    text(weight: 600, size: base-size * 0.92, tracking: 0.22em, fill: accent, upper(title))
  } else {
    text(weight: 600, size: base-size * 1.12, fill: accent, title)
  }
  block(above: gap * 1.2, below: 0.75em, width: 100%, {
    label
    if theme.headingStyle == "underline" {
      v(0.35em, weak: true)
      line(length: 100%, stroke: 0.4pt + accent.lighten(50%))
    }
  })
}

#let marker = bullet-marker(theme, dot: text(fill: accent)[•])

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.04, body),
  company: body => text(weight: 600, size: base-size * 1.12, tracking: 0.02em, body),
  meta: body => text(fill: muted, size: base-size * 0.92, tracking: 0.03em, body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.15,
)

// ---------------------------------------------------------------- header

#align(center, {
  text(size: base-size * 2.1, weight: 500, tracking: 0.18em, upper(basics.fullName))
  if nonempty(basics.headline) {
    v(0.5em, weak: true)
    text(size: base-size * 1.0, tracking: 0.12em, fill: accent, upper(basics.headline))
  }
  let bits = contact-bits(basics)
  if bits.len() > 0 {
    v(0.7em, weak: true)
    text(size: base-size * 0.9, fill: muted, bits.join(h(0.7em) + text(fill: accent.lighten(40%))[|] + h(0.7em)))
  }
})
#v(0.9em, weak: true)
#line(length: 100%, stroke: 0.5pt + luma(170))

#if rich-nonempty(basics.summary) {
  v(0.9em, weak: true)
  // The lead: a step up in size, so it is read first and read whole.
  set text(size: base-size * 1.1)
  set par(leading: leading * 1.05)
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
