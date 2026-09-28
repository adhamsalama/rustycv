// Sidebar — two columns. A narrow rail tinted with the accent carries the
// things a reader scans rather than reads (contact details, skills, languages,
// interests); the wide column carries the story (summary, experience,
// education, projects and everything else).
//
// Which section goes where is decided by its *kind*, not its position, so the
// user's ordering still holds within each column. The rail is a block sized to
// its content rather than a page-high band, so it ends where its last group
// does and never paints an empty strip down a second page.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
#let rail-fill = accent.lighten(91%)
#let muted = luma(95)
#let base-size = theme.fontSizePt * 1pt
// Shared by paragraphs and list items so both follow the line-height control.
#let leading = theme.lineHeight * 0.65em
#let gap = theme.sectionGapMm * 1mm

#let rail-kinds = ("skills", "languages", "interests")

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

#let heading-label(title, size) = {
  let caps = theme.headingStyle == "caps"
  if caps {
    text(weight: 700, size: size * 0.9, tracking: 0.1em, fill: accent, upper(title))
  } else {
    text(weight: 700, size: size, fill: accent, title)
  }
}

#let main-heading(title) = block(above: gap, below: 0.55em, width: 100%, {
  heading-label(title, base-size * 1.12)
  if theme.headingStyle == "underline" {
    v(0.25em, weak: true)
    line(length: 100%, stroke: 0.6pt + accent.lighten(40%))
  }
})

// The rail's headings are a step smaller; its column is a third the width.
#let rail-heading(title) = block(above: gap, below: 0.5em, width: 100%, {
  heading-label(title, base-size * 1.0)
  if theme.headingStyle == "underline" {
    v(0.25em, weak: true)
    line(length: 100%, stroke: 0.6pt + accent.lighten(40%))
  }
})

// Stacked for a narrow column: the group's name, then its items beneath.
#let rail-skills(groups, style, ..) = {
  let t = style.tight
  for g in groups {
    let values = g.at("items", default: ()).filter(s => nonempty(s))
    let name = g.at("name", default: "")
    if values.len() == 0 and not nonempty(name) { continue }
    block(above: 0.7em * t, below: 0.7em * t, {
      if nonempty(name) { block(below: style.leading, text(weight: 600, name)) }
      values.join(", ")
    })
  }
}

#let rail-list(render-name) = (items, style, ..) => {
  for it in items.filter(it => nonempty(it.name)) {
    block(above: style.leading, below: style.leading, render-name(it))
  }
}

#let marker = bullet-marker(theme)

#let base-style = (
  title: body => text(weight: 600, size: base-size * 1.02, body),
  company: body => text(weight: 700, size: base-size * 1.1, body),
  meta: body => text(fill: muted, size: base-size * 0.93, body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.0,
)

#let main-style = base-style + (heading: main-heading)
#let rail-style = base-style + (
  heading: rail-heading,
  sections: (
    skills: rail-skills,
    languages: rail-list(it => {
      text(weight: 600, it.name)
      if nonempty(it.at("level", default: "")) { linebreak() + text(fill: muted, size: base-size * 0.93, it.level) }
    }),
    interests: rail-list(it => it.name),
  ),
)

#let rail-sections = cv.sections.filter(s => s.kind in rail-kinds)
#let main-sections = cv.sections.filter(s => s.kind not in rail-kinds)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, {
  text(size: base-size * 2.1, weight: 700, basics.fullName)
  if nonempty(basics.headline) {
    v(0.3em, weak: true)
    text(size: base-size * 1.15, fill: accent, weight: 500, basics.headline)
  }
})
#v(1.1em, weak: true)

// ---------------------------------------------------------------- columns

#let bits = contact-bits(basics)
#let rail = {
  if bits.len() > 0 {
    rail-heading("Contact")
    set text(size: base-size * 0.93)
    for b in bits { block(above: leading, below: leading, b) }
  }
  render-sections((sections: rail-sections), rail-style)
}

#let main = {
  if rich-nonempty(basics.summary) {
    // Opens the column without a heading, level with the rail's first line.
    block(above: 0pt, below: gap, rich(basics.summary, gap: leading, marker: marker))
  }
  render-sections((sections: main-sections), main-style)
}

// Inset in em so the tint's margin grows with the type.
#grid(
  columns: (31%, 1fr),
  column-gutter: 1.5em,
  block(width: 100%, fill: rail-fill, radius: 3pt, inset: (x: 0.9em, y: 0.9em), rail),
  main,
)
