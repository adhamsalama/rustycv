// Academic — a curriculum vitae rather than a résumé. Sober: a centred serif
// name, colour on links only, and dates hung in a gutter to the left of every
// dated entry so a long record reads down the page as a chronology. It is
// designed to run to several pages, and from the second one on the footer
// carries the name and a page number.
//
// The date gutter is this template's own entry geometry — the shared one puts
// dates on the right — so experience, education and certifications are
// rendered here, and everything else is the shared renderer indented to the
// text column.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let display = "Source Serif 4"
#let accent = rgb(theme.accent)
#let muted = luma(90)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.68em
#let gap = theme.sectionGapMm * 1mm
// Wide enough for "Sep 2021 – Present" at the meta size.
#let gutter = 9.2em

#set document(
  title: join-parts((basics.fullName, basics.headline), sep: " – "),
  author: basics.fullName,
)

#set page(
  paper: if theme.page == "letter" { "us-letter" } else { "a4" },
  margin: theme.marginMm * 1mm,
  // Only on a CV that needs one: a single page is not numbered.
  footer: context {
    if counter(page).final().first() > 1 {
      set text(size: base-size * 0.85, fill: muted)
      basics.fullName
      h(1fr)
      counter(page).display("1 / 1", both: true)
    }
  },
)

#set text(font: theme.fontFamily, size: base-size, lang: "en", fallback: true)
#set par(leading: leading, justify: false, spacing: theme.lineHeight * 0.78em)
#show link: set text(fill: accent)

#let section-heading(title) = {
  let label = if theme.headingStyle == "caps" {
    text(font: display, weight: 600, size: base-size * 1.02, tracking: 0.12em, upper(title))
  } else {
    text(font: display, weight: 600, size: base-size * 1.25, title)
  }
  block(above: gap, below: 0.65em, width: 100%, sticky: true, {
    label
    if theme.headingStyle == "underline" {
      v(0.3em, weak: true)
      line(length: 100%, stroke: 0.5pt + luma(120))
    }
  })
}

#let marker = bullet-marker(theme)

#let meta(body) = text(fill: muted, size: base-size * 0.92, body)

// A date in the gutter, the entry beside it.
#let dated(date, body) = grid(
  columns: (gutter, 1fr),
  column-gutter: 0.8em,
  meta(date),
  body,
)

#let experience(items, style, section) = {
  let options = experience-options(section)
  let t = style.tight * style.line-height
  for group in group-by-company(items, enabled: options.group-promotions) {
    let grouped = group.items.len() > 1
    if grouped {
      block(above: 0.95em * t, below: style.leading, dated([], (style.company)(maybe-link(
        group.items.first().at("companyUrl", default: ""),
        group.company,
      ))))
    }
    for (i, it) in group.items.enumerate() {
      let heading = if grouped { (style.title)(it.role) } else {
        (style.title)(entry-line(it, options.order, maybe-link))
      }
      block(
        above: if not grouped { 0.95em * t } else if i == 0 { style.leading } else { 0.85em * t },
        below: 0pt,
        dated(fmt-range(it.start, it.end, current: it.at("current", default: false)), {
          heading
          if nonempty(it.at("location", default: "")) {
            block(above: style.leading, below: 0pt, meta(it.location))
          }
          let bullets = it.at("bullets", default: ())
          if rich-nonempty(bullets) {
            block(above: style.leading, below: 0pt, rich(bullets, gap: style.leading, marker: style.marker))
          }
        }),
      )
    }
  }
}

#let education(items, style, ..) = {
  let t = style.tight
  for it in items {
    block(above: 0.85em * t, below: 0pt, dated(
      fmt-range(it.start, it.end, current: it.at("current", default: false)),
      {
        (style.title)(it.degree)
        if nonempty(it.institution) { linebreak() + emph(it.institution) }
        if rich-nonempty(it.at("description", default: "")) {
          block(above: style.leading, below: 0pt, rich(it.description, gap: style.leading, marker: style.marker))
        }
      },
    ))
  }
}

#let certifications(items, style, ..) = {
  let t = style.tight
  for it in items {
    block(above: 0.6em * t, below: 0pt, dated(fmt-date(it.at("date", default: none)), {
      maybe-link(it.at("url", default: ""), (style.title)(it.name))
      if nonempty(it.at("issuer", default: "")) { [, ] + emph(it.issuer) }
    }))
  }
}

// Undated sections still line up with the text column rather than the gutter,
// so the page keeps one left edge for text.
#let indented(render) = (items, style, ..) => pad(left: gutter + 0.8em, render(items, style))

#let style = (
  heading: section-heading,
  title: body => text(weight: 600, body),
  company: body => text(weight: 700, size: base-size * 1.05, body),
  meta: meta,
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.0,
  sections: (
    experience: experience,
    education: education,
    certifications: certifications,
    skills: indented(skills-section),
    projects: indented(projects-section),
    languages: indented(languages-section),
    interests: indented(interests-section),
    references: indented(references-section),
    custom: indented(custom-section),
  ),
)

// ---------------------------------------------------------------- header

#align(center, {
  text(font: display, size: base-size * 2.2, weight: 600, basics.fullName)
  if nonempty(basics.headline) {
    v(0.4em, weak: true)
    text(font: display, size: base-size * 1.1, style: "italic", basics.headline)
  }
  let bits = contact-bits(basics)
  if bits.len() > 0 {
    v(0.6em, weak: true)
    text(size: base-size * 0.92, bits.join([ #sym.dot.c ]))
  }
})

#if rich-nonempty(basics.summary) {
  v(1em, weak: true)
  set par(justify: true)
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
