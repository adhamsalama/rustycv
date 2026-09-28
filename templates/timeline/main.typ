// Timeline — experience drawn as a career path. A thin rail runs down the left
// of each employer with a node at every role, so a run of promotions reads as
// one continuous line with several stops on it, and a change of employer is a
// break in the line. Everything outside experience uses the shared layout,
// indented to the rail so the page keeps one left edge for text.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let rail-ink = accent.lighten(55%)
#let muted = luma(95)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.65em
#let gap = theme.sectionGapMm * 1mm
// Distance from the rail to the text beside it, in em so it grows with type.
#let rail-inset = 1.3em

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

#let section-heading(title) = {
  let caps = theme.headingStyle == "caps"
  block(above: gap, below: 0.6em, width: 100%, {
    text(
      weight: 700,
      size: base-size * (if caps { 0.98 } else { 1.12 }),
      tracking: if caps { 0.08em } else { 0em },
      fill: accent,
      if caps { upper(title) } else { title },
    )
    if theme.headingStyle == "underline" {
      v(0.25em, weak: true)
      line(length: 100%, stroke: 0.6pt + accent.lighten(40%))
    }
  })
}

#let marker = bullet-marker(theme)

// A node on the rail, centred on it and level with the first line of `body`.
// `place` takes it out of the flow, so the entry lays out exactly as it would
// without one.
#let node(radius, filled) = place(
  top + left,
  dx: -rail-inset - radius,
  dy: 0.5em - radius,
  circle(
    radius: radius,
    fill: if filled { accent } else { white },
    stroke: 1pt + accent,
  ),
)

#let experience(items, style, section) = {
  let options = experience-options(section)
  let t = style.tight * style.line-height
  for group in group-by-company(items, enabled: options.group-promotions) {
    let grouped = group.items.len() > 1
    // One block per employer, so the rail is one unbroken stroke across every
    // role there and breaks where the employer changes.
    block(
      above: 0.95em * t,
      below: 0pt,
      width: 100%,
      stroke: (left: 1pt + rail-ink),
      inset: (left: rail-inset, bottom: 0.1em),
      {
        if grouped {
          block(above: 0pt, below: style.leading, {
            node(0.34em, true)
            (style.company)(maybe-link(group.items.first().at("companyUrl", default: ""), group.company))
          })
        }
        for (i, it) in group.items.enumerate() {
          let heading = if grouped { (style.title)(it.role) } else {
            (style.title)(entry-line(it, options.order, maybe-link))
          }
          block(
            above: if not grouped or i == 0 { style.leading } else { 0.85em * t },
            below: style.leading,
            {
              // Roles under a company are the smaller, hollow stops.
              if grouped { node(0.24em, false) } else { node(0.34em, true) }
              row(heading, (style.meta)(fmt-range(it.start, it.end, current: it.at("current", default: false))))
            },
          )
          if nonempty(it.at("location", default: "")) {
            block(spacing: 0.3em * t, (style.meta)(it.location))
          }
          rich(it.at("bullets", default: ()), gap: style.leading, marker: style.marker)
        }
      },
    )
  }
}

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  company: body => text(weight: 700, size: base-size * 1.1, body),
  meta: body => text(fill: muted, size: base-size * 0.93, body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.0,
  sections: (experience: experience),
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

#if rich-nonempty(basics.summary) {
  v(0.7em, weak: true)
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
