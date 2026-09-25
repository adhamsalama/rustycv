// FlowCV — a faithful reproduction of FlowCV's default single-column resume.
//
// Measurements were taken from a published FlowCV resume's rendered markup, so
// the numbers here are transcribed rather than invented. FlowCV's own reference
// values are noted alongside each one. Everything is expressed relative to the
// theme's base size, so the type-size slider still works; at the default 9pt
// the output matches FlowCV one-to-one.
//
// Reference: 210×297mm, padding 9.47/10/7.88mm, Source Sans Pro 9pt/15px.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let base = theme.fontSizePt * 1pt
#let rule-grey = rgb("#cccccc")

// FlowCV tints the heading band with 7% black regardless of accent. Deriving it
// from the accent instead keeps the band neutral for the near-black default and
// lets a coloured accent carry through.
#let band = accent.lighten(93%)

#set document(
  title: join-parts((basics.fullName, basics.headline), sep: " – "),
  author: basics.fullName,
)

#set page(
  paper: if theme.page == "letter" { "us-letter" } else { "a4" },
  // FlowCV: 9.47083mm top, 10mm sides, 7.88333mm bottom.
  margin: (
    x: theme.marginMm * 1mm,
    top: theme.marginMm * 0.947mm,
    bottom: theme.marginMm * 0.788mm,
  ),
)

#set text(font: theme.fontFamily, size: base, lang: "en", fallback: true, fill: black)
// FlowCV: 15px line box on a 9pt (12px) font — a 1.25 ratio.
#set par(leading: theme.lineHeight * 0.5em, justify: false, spacing: theme.lineHeight * 0.5em)
#show link: set text(fill: black)

// ---------------------------------------------------------------- icons

// Icons ship as SVG so they can be tinted here: Font Awesome's paths inherit
// `fill` from the root element, which is not set in the shipped files.
#let icon(name, size: 1em, fill: black, shift: 0.16em) = box(
  baseline: shift,
  image(
    bytes(read("/icons/" + name + ".svg").replace("<svg ", "<svg fill=\"" + fill.to-hex() + "\" ")),
    format: "svg",
    height: size,
  ),
)

#let section-icons = (
  experience: "briefcase",
  education: "graduation-cap",
  skills: "brain",
  projects: "rocket",
  certifications: "certificate",
  languages: "language",
  interests: "heart",
  references: "user",
)

// Recognise the common link targets by label so they get their own mark, the
// way FlowCV does; anything else falls back to a generic chain.
#let link-icon(label) = {
  let l = lower(label).trim()
  if l == "github" { "github" }
  else if l == "linkedin" { "linkedin-in" }
  else { "link" }
}

// The small chain that FlowCV puts after any linked name.
#let link-mark = icon("link", size: 0.78em, shift: 0.02em)

#let linked(url, body) = if nonempty(url) {
  link(url, body + h(0.15em) + link-mark)
} else { body }

// ---------------------------------------------------------------- header

// FlowCV runs the name and the italic job title inline in one centred block,
// so they share a line when they fit and wrap naturally when they don't.
#align(center, block(width: 100%, inset: (bottom: 0.53em), {
  text(size: base * 1.889, weight: 700, basics.fullName)
  if nonempty(basics.headline) {
    text(size: base * 1.889, "  ")
    text(size: base * 1.556, style: "italic", weight: "regular", basics.headline)
  }
}))

#let contact-items = {
  let items = ()
  if nonempty(basics.location) {
    items.push((icon: "location-dot", body: [#basics.location], url: ""))
  }
  if nonempty(basics.email) {
    items.push((icon: "envelope", body: [#basics.email], url: "mailto:" + basics.email))
  }
  if nonempty(basics.phone) {
    items.push((icon: "phone", body: [#basics.phone], url: "tel:" + basics.phone))
  }
  for l in basics.at("links", default: ()) {
    if nonempty(l.url) {
      let label = if nonempty(l.label) { l.label } else { l.url }
      items.push((icon: link-icon(label), body: [#label], url: l.url))
    }
  }
  items
}

#if contact-items.len() > 0 {
  align(center, block(width: 100%, inset: (bottom: 0.3em), {
    // A wrapping, centred row: icon + 0.5625em + value, separated by 1em.
    let rendered = contact-items.map(it => box({
      icon(it.icon, size: 0.95em, fill: accent)
      h(0.5625em)
      if nonempty(it.url) { link(it.url, it.body) } else { it.body }
    }))
    set par(leading: 0.7em)
    rendered.join(h(1em))
  }))
}

#if nonempty(basics.summary) {
  block(width: 100%, inset: (bottom: 0.4em), basics.summary)
}

// ------------------------------------------------------------- components

// The full-width tinted band that heads every section.
#let fc-heading(title, kind) = block(
  width: 100%,
  above: 0.95em,
  below: 0.6em,
  fill: band,
  radius: 2pt,
  inset: (y: base * 0.25, x: 0.35em),
  align(center, {
    icon(section-icons.at(kind, default: "star"), size: base * 1.1, fill: accent)
    h(0.5em)
    text(weight: 700, size: base * 1.111, tracking: 0.0375em, fill: accent, upper(title))
  }),
)

// FlowCV's 55/45 split: headline left, date right-aligned in the rest.
#let fc-row(left-side, right-side) = grid(
  columns: (55%, 45%),
  left-side,
  align(right + top, right-side),
)

#let fc-date(it) = text(
  weight: "regular",
  fmt-range(it.at("start", default: none), it.at("end", default: none), current: it.at("current", default: false)),
)

// A 4px dot at FlowCV's 96dpi reference, i.e. a third of the text size.
#let fc-bullets(items) = {
  let items = items.filter(b => nonempty(b))
  if items.len() == 0 { return }
  block(inset: (left: 0.8em), width: 100%, above: 0.32em, below: 0.32em, {
    set list(
      marker: box(baseline: -0.24em, circle(radius: base * 0.16, fill: black)),
      indent: 0pt,
      body-indent: 0.42em,
      spacing: 0.36em,
    )
    list(..items.map(b => [#b]))
  })
}

// Roles under a shared employer sit behind a hairline, indented from it.
#let fc-indented(body) = block(
  width: 100%,
  inset: (left: 0.8em),
  stroke: (left: 1pt + rule-grey),
  outset: (left: 0pt),
  body,
)

// ---------------------------------------------------------------- sections

#let fc-experience(items) = {
  for group in group-by-company(items) {
    let grouped = group.items.len() > 1
    if grouped {
      // A run of roles at one employer: the company is named once, above.
      block(width: 100%, below: 0.25em, text(weight: 700, group.company))
      for it in group.items {
        fc-indented({
          fc-row(text(weight: 700, it.role), fc-date(it))
          fc-bullets(it.at("bullets", default: ()))
        })
        v(0.5em, weak: true)
      }
    } else {
      // A one-off role reads inline as "Backend Engineer, Wuilt" with the
      // company in regular weight — no employer heading, no rule.
      let it = group.items.first()
      let heading = if nonempty(it.at("company", default: "")) {
        text(weight: 700, it.role + ", ") + linked(
          it.at("companyUrl", default: ""),
          text(weight: "regular", it.company),
        )
      } else {
        text(weight: 700, it.role)
      }
      block(width: 100%, below: 0.1em, fc-row(heading, fc-date(it)))
      fc-bullets(it.at("bullets", default: ()))
      v(0.5em, weak: true)
    }
  }
}

#let fc-education(items) = {
  for it in items {
    block(width: 100%, below: 0.1em, fc-row(
      text(weight: 700, it.degree + if nonempty(it.at("institution", default: "")) { ", " } else { "" })
        + text(weight: "regular", it.at("institution", default: "")),
      fc-date(it),
    ))
    if nonempty(it.at("description", default: "")) {
      block(width: 100%, above: 0.3em, it.description)
    }
    v(0.5em, weak: true)
  }
}

#let fc-skills(groups) = {
  for g in groups {
    let values = g.at("items", default: ()).filter(s => nonempty(s))
    let name = g.at("name", default: "")
    if values.len() == 0 and not nonempty(name) { continue }
    block(width: 100%, {
      if nonempty(name) { text(weight: 700, name + ":") + sym.space.nobreak }
      values.join(", ")
    })
  }
}

#let fc-projects(items) = {
  for it in items {
    block(width: 100%, below: 6pt, {
      linked(it.at("url", default: ""), text(weight: 700, it.name))
      if nonempty(it.at("description", default: "")) { [, ] + it.description }
      let tech = it.at("tech", default: ()).filter(x => nonempty(x))
      if tech.len() > 0 { [ ] + text(style: "italic", tech.join(", ")) }
    })
  }
}

#let fc-certifications(items) = {
  for it in items {
    block(width: 100%, below: 0.1em, fc-row(
      linked(it.at("url", default: ""), text(weight: 700, it.name))
        + if nonempty(it.at("issuer", default: "")) { text(weight: "regular", ", " + it.issuer) } else { [] },
      text(weight: "regular", fmt-date(it.at("date", default: none))),
    ))
    v(0.5em, weak: true)
  }
}

#let fc-inline-list(items, fmt) = {
  let parts = items.filter(it => nonempty(it.at("name", default: ""))).map(fmt)
  if parts.len() > 0 { block(width: 100%, parts.join(", ")) }
}

#let fc-references(items) = {
  for it in items {
    block(width: 100%, {
      linked(it.at("url", default: ""), text(weight: 700, it.name))
      let role = it.at("title", default: "")
      if nonempty(role) { [, ] + text(style: "italic", role) }
      if nonempty(it.at("company", default: "")) { [, ] + it.company }
      let contact = join-parts((it.at("email", default: ""), it.at("phone", default: "")))
      if nonempty(contact) { [ ] + text(fill: luma(100), contact) }
    })
  }
}

// ---------------------------------------------------------------- document

#for section in cv.sections {
  let items = section-items(section)
  if not section.at("visible", default: true) or items.len() == 0 { continue }

  fc-heading(section.title, section.kind)

  if section.kind == "experience" { fc-experience(items) }
  else if section.kind == "education" { fc-education(items) }
  else if section.kind == "skills" { fc-skills(items) }
  else if section.kind == "projects" { fc-projects(items) }
  else if section.kind == "certifications" { fc-certifications(items) }
  else if section.kind == "languages" {
    fc-inline-list(items, it => if nonempty(it.at("level", default: "")) {
      it.name + " (" + it.level + ")"
    } else { it.name })
  }
  else if section.kind == "interests" { fc-inline-list(items, it => it.name) }
  else if section.kind == "references" { fc-references(items) }
}
