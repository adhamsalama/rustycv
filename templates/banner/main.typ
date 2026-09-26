// Banner — the name and contact details reversed out of a filled accent block
// across the top, and the accent then repeated as a small bar beside every
// section heading.
//
// It is the one template where the accent colour is the page's structure rather
// than its trim, so it is also the one that has to cope with any accent: the
// text on the band picks itself to stay legible on whatever colour it lands on.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let muted = luma(100)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.68em
#let gap = theme.sectionGapMm * 1mm

// White on a dark band, black on a light one. Rec. 601 luma is a crude measure
// of brightness, but it only has to pick between two answers, and picking
// wrongly is the difference between a readable name and an invisible one.
#let (accent-r, accent-g, accent-b, ..) = accent.rgb().components()
#let accent-bright = 0.299 * accent-r + 0.587 * accent-g + 0.114 * accent-b
#let on-band = if accent-bright > 58% { black } else { white }
// The same ink stepped back for secondary lines, in whichever direction the
// band leaves room.
#let on-band-soft = if accent-bright > 58% { black.lighten(38%) } else { white.darken(18%) }

// The band pushes people towards vivid accents, and a yellow that reads well as
// a filled block is invisible as 8pt text on white. So everything drawn *on the
// paper* uses the accent darkened back to a legible weight; only the band
// itself gets the colour as picked.
#let accent-ink = if accent-bright > 58% { accent.darken(45%) } else { accent }

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
#show link: set text(fill: accent-ink)

// The bar is this template's mark and is always there; `headingStyle` chooses
// the case, and `underline` adds the rule on top of it.
#let section-heading(title) = {
  let caps = theme.headingStyle == "caps"
  let label = text(
    weight: 700,
    size: base-size * (if caps { 0.95 } else { 1.1 }),
    tracking: if caps { 0.09em } else { 0em },
    fill: accent-ink,
    if caps { upper(title) } else { title },
  )
  block(above: gap, below: 0.55em, width: 100%, {
    grid(
      columns: (auto, auto),
      column-gutter: 0.55em,
      align: horizon,
      rect(fill: accent-ink, width: 0.28em, height: 0.95em, radius: 1pt),
      label,
    )
    if theme.headingStyle == "underline" {
      v(0.28em, weak: true)
      line(length: 100%, stroke: 0.6pt + accent-ink.lighten(55%))
    }
  })
}

#let marker = bullet-marker(theme)

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  // An employer heading sits a step above an entry title so the reader can tell
  // "who I worked for" from "what I did there" at a glance.
  company: body => text(weight: 700, size: base-size * 1.12, body),
  meta: body => text(fill: muted, size: base-size * 0.92, style: "italic", body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.08,
)

// ---------------------------------------------------------------- header

// Inset in em, like everything else here, so the band grows with the type
// rather than clamping around it as the size slider goes up.
#block(
  width: 100%,
  fill: accent,
  radius: 3pt,
  inset: (x: 0.9em, y: 0.85em),
  below: 0.9em,
  {
    text(size: base-size * 1.9, weight: 700, fill: on-band, basics.fullName)
    if nonempty(basics.headline) {
      v(0.4em, weak: true)
      text(size: base-size * 1.1, weight: 500, fill: on-band-soft, basics.headline)
    }
    let bits = contact-bits(basics)
    if bits.len() > 0 {
      v(0.45em, weak: true)
      // Links on the band take the band's ink, not the accent they sit on.
      show link: set text(fill: on-band-soft)
      text(
        size: base-size * 0.92,
        fill: on-band-soft,
        bits.join(text(fill: on-band-soft)[ #sym.dot.c ]),
      )
    }
  },
)

#if rich-nonempty(basics.summary) {
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
