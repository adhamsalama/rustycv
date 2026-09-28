// Plain — built for applicant-tracking portals (Workday, Taleo, government
// systems) that mangle anything clever: one column, black text, standard
// spelled-out headings, every contact detail on a line of its own, no rules,
// bands, icons or tables. The accent reaches the links and nothing else, so a
// parser sees exactly what a person reads.

#import "/common.typ": *

#let cv = json("/data.json")
#let theme = cv.theme
#let basics = cv.basics

#let accent = rgb(theme.accent)
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
// Underlined as well as coloured, so a link survives a black-and-white print.
#show link: it => underline(text(fill: accent, it))

// Parsers find sections by their words, so the heading is the title and
// nothing else. `underline` still draws a rule — a plain one, in black.
#let section-heading(title) = {
  let caps = theme.headingStyle == "caps"
  block(above: gap, below: 0.5em, width: 100%, {
    text(weight: 700, size: base-size * (if caps { 1.0 } else { 1.12 }), if caps { upper(title) } else { title })
    if theme.headingStyle == "underline" {
      v(0.2em, weak: true)
      line(length: 100%, stroke: 0.5pt + black)
    }
  })
}

#let marker = bullet-marker(theme)

#let style = (
  heading: section-heading,
  title: body => text(weight: 700, body),
  company: body => text(weight: 700, size: base-size * 1.05, body),
  meta: body => text(body),
  marker: marker,
  leading: leading,
  line-height: theme.lineHeight,
  tight: 1.0,
)

// ---------------------------------------------------------------- header

#block(spacing: 0pt, text(size: base-size * 1.6, weight: 700, basics.fullName))
#if nonempty(basics.headline) {
  v(0.3em, weak: true)
  text(size: base-size * 1.05, basics.headline)
}

// Labelled, one per line: "Email:" is what a portal's field matcher looks for.
#let lines = ()
#if nonempty(basics.location) { lines.push([Location: #basics.location]) }
#if nonempty(basics.email) { lines.push([Email: #link("mailto:" + basics.email, basics.email)]) }
#if nonempty(basics.phone) { lines.push([Phone: #link("tel:" + basics.phone, basics.phone)]) }
#for l in basics.at("links", default: ()) {
  if nonempty(l.url) {
    // The address itself, not a label: a parser cannot follow "GitHub".
    lines.push([#(if nonempty(l.label) { l.label + ": " })#link(l.url, l.url)])
  }
}
#if lines.len() > 0 {
  v(0.5em, weak: true)
  block(spacing: 0pt, lines.join(linebreak()))
}

#if rich-nonempty(basics.summary) {
  section-heading("Summary")
  rich(basics.summary, gap: leading, marker: marker)
}

#render-sections(cv, style)
