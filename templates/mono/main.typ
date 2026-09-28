// Mono — a README in print. Name, headings and dates are set in IBM Plex Mono,
// headings read like Markdown (`## experience`), and skills and a project's
// stack are inline code chips. The body stays in the proportional face the
// theme picks, because a page of monospace prose is tiring to read, and the
// layout stays a single column of real text so a parser loses nothing.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let mono = "IBM Plex Mono"
#let accent = rgb(theme.accent)
#let muted = luma(105)
#let chip-fill = luma(238)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.65em
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

// Mono at the body's size looks a size larger, so it is set a notch down.
#let code(body, ..args) = text(font: mono, size: base-size * 0.9, ..args, body)

// Inset in em so a chip grows with the type; the radius is a pt, which is
// geometry rather than spacing.
#let chip(body) = box(
  fill: chip-fill,
  radius: 2pt,
  inset: (x: 0.35em, y: 0.2em),
  outset: (y: 0.05em),
  code(body),
)

#let section-heading(title) = {
  let caps = theme.headingStyle == "caps"
  block(above: gap, below: 0.55em, width: 100%, {
    code(fill: accent, weight: 600, size: base-size * 1.02)[\#\# ]
    code(weight: 600, size: base-size * 1.02, if caps { upper(title) } else { lower(title) })
    if theme.headingStyle == "underline" {
      v(0.25em, weak: true)
      // The run of `=` Markdown's setext headings use, drawn as a rule.
      line(length: 100%, stroke: (paint: accent.lighten(40%), thickness: 0.6pt, dash: "dashed"))
    }
  })
}

#let marker = bullet-marker(theme, dot: code(fill: accent)[-])

#let skills(groups, style, ..) = {
  let t = style.tight
  for g in groups {
    let values = g.at("items", default: ()).filter(s => nonempty(s))
    let name = g.at("name", default: "")
    if values.len() == 0 and not nonempty(name) { continue }
    block(above: 0.55em * t, below: 0.55em * t, {
      if nonempty(name) { text(weight: 600, name) + h(0.6em) }
      values.map(chip).join(h(0.3em))
    })
  }
}

#let projects(items, style, ..) = {
  let t = style.tight
  for it in items {
    let description = it.at("description", default: "")
    let inline-description = rich-inlineable(description)
    block(above: 0.6em * t, below: 0.6em * t, {
      maybe-link(it.at("url", default: ""), (style.title)(it.name))
      if inline-description and rich-nonempty(description) { [ — ] + rich-inline(description) }
      let tech = it.at("tech", default: ()).filter(x => nonempty(x))
      if tech.len() > 0 { h(0.5em) + tech.map(chip).join(h(0.3em)) }
      if not inline-description {
        block(above: style.leading, below: 0pt, rich(description, gap: style.leading, marker: style.marker))
      }
    })
  }
}

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, size: base-size * 1.02, body),
  company: body => text(weight: 700, size: base-size * 1.1, body),
  meta: body => code(fill: muted, body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 0.95,
  sections: (skills: skills, projects: projects),
)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, {
  code(size: base-size * 2.0, weight: 700, basics.fullName)
  code(size: base-size * 2.0, weight: 400, fill: accent)[\_]
})
#if nonempty(basics.headline) {
  v(0.35em, weak: true)
  code(size: base-size * 1.05, fill: accent)[\/\/ #basics.headline]
}

#let bits = contact-bits(basics)
#if bits.len() > 0 {
  v(0.6em, weak: true)
  code(bits.join(code(fill: muted)[ | ]))
}

#if rich-nonempty(basics.summary) {
  v(0.8em, weak: true)
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
