// Shared vocabulary for every RustyCV template.
//
// Templates import this, then layer their own styling on top. Anything that is
// about *the data* (how a date range reads, how consecutive roles at the same
// employer group together) lives here so all templates agree; anything that is
// about *the look* stays in the template.

#let months = (
  "Jan", "Feb", "Mar", "Apr", "May", "Jun",
  "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
)

// A `{year, month?}` dict as text. Month-less dates render as the bare year,
// which is what education entries ("2017 – 2022") want.
#let fmt-date(d) = {
  if d == none { return "" }
  let m = d.at("month", default: none)
  if m == none or m < 1 or m > 12 { str(d.year) } else { months.at(m - 1) + " " + str(d.year) }
}

// "Jan 2026 – Present", "2017 – 2022", or a lone endpoint if only one is set.
#let fmt-range(start, end, current: false) = {
  let a = fmt-date(start)
  let b = if current { "Present" } else { fmt-date(end) }
  if a == "" and b == "" { "" } else if a == "" { b } else if b == "" { a } else { a + " – " + b }
}

// Rich text arrives in one of three forms — a bare string (one unstyled
// paragraph), an array of runs (one styled paragraph), or an array of blocks.
// `rich-blocks` normalises all three so nothing below has to know which it got.
#let rich-blocks(value) = {
  if value == none { return () }
  if type(value) == str {
    return if value == "" { () } else { ((kind: "paragraph", runs: ((text: value),)),) }
  }
  if type(value) != array or value.len() == 0 { return () }
  // A block carries `runs`; a run carries `text`. That is the whole difference.
  if "runs" in value.first() {
    return value.map(b => (
      kind: b.at("kind", default: "paragraph"),
      runs: b.at("runs", default: ()),
    ))
  }
  ((kind: "paragraph", runs: value),)
}

// One block's runs as inline content.
#let rich-runs(runs) = {
  let out = []
  for run in runs {
    let body = run.at("text", default: "")
    if body == "" { continue }
    // A soft line break inside a paragraph is stored as a newline in the run's
    // text, and Typst breaks the line on a newline inside a text value — unlike
    // a newline written in markup, which is ordinary whitespace.
    // `a_soft_line_break_starts_a_new_line` pins that.
    let piece = [#body]
    // Applied outermost-last so a run that is bold *and* a link renders as both.
    if run.at("bold", default: false) { piece = strong(piece) }
    if run.at("italic", default: false) { piece = emph(piece) }
    if run.at("underline", default: false) { piece = underline(piece) }
    let url = run.at("link", default: "")
    if url != "" { piece = link(url, piece) }
    out += piece
  }
  out
}

// The plain text behind rich text, for emptiness checks. `sum` rather than
// `join`, which answers `none` for an empty array and would poison every
// caller's `.trim()`.
#let block-text(b) = b.runs.map(r => r.at("text", default: "")).sum(default: "")

#let rich-text(value) = rich-blocks(value).map(block-text).intersperse("\n").sum(default: "")

#let rich-nonempty(value) = rich-text(value).trim() != ""

// The blocks that would actually render.
#let rich-filled(value) = rich-blocks(value).filter(b => block-text(b).trim() != "")

// True when the whole value fits on a line already in progress: at most one
// paragraph, no list markers, no line breaks. A project description reads after
// the project's name when this holds and drops below it when it does not.
#let rich-inlineable(value) = {
  let blocks = rich-filled(value)
  if blocks.len() > 1 { return false }
  blocks.all(b => b.kind == "paragraph" and not b.runs.any(r => "\n" in r.at("text", default: "")))
}

// Rich text as inline content, for the places that have no room for blocks —
// a list item's body, or a project's one-line description.
#let rich-inline(value) = rich-filled(value).map(b => rich-runs(b.runs)).intersperse([ ]).sum(default: [])

// Rich text as block content: paragraphs and lists stacked, with consecutive
// list items of one kind gathered into a single list so their markers line up.
//
// `gap` is the caller's line step. Items inside a list sit exactly that far
// apart, the same as an entry's highlights; the break *between* blocks is two
// steps, because one step is what a line already costs and a paragraph break
// set to it would be indistinguishable from a wrapped line. Everything is
// measured in line steps, so the type-size and line-height controls move it.
#let rich(value, gap: 0.5em, marker: [•], body-indent: 0.45em) = {
  let blocks = rich-filled(value)
  if blocks.len() == 0 { return }

  // One paragraph is the overwhelmingly common case and predates blocks
  // entirely: hand it back as plain inline content so that the weak spacing a
  // template sets around it still collapses the way it always has.
  if blocks.len() == 1 and blocks.first().kind == "paragraph" {
    return rich-runs(blocks.first().runs)
  }

  let groups = ()
  for b in blocks {
    if groups.len() > 0 and groups.last().kind == b.kind and b.kind != "paragraph" {
      groups.last().items.push(b.runs)
    } else {
      groups.push((kind: b.kind, items: (b.runs,)))
    }
  }

  for (i, g) in groups.enumerate() {
    let body = if g.kind == "bullet" {
      set list(marker: marker, indent: 0pt, body-indent: body-indent, spacing: gap)
      list(..g.items.map(rich-runs))
    } else if g.kind == "numbered" {
      set enum(indent: 0pt, body-indent: body-indent, spacing: gap)
      enum(..g.items.map(rich-runs))
    } else {
      rich-runs(g.items.first())
    }
    // Only the gaps *between* blocks belong to rich text; what surrounds the
    // whole value is the caller's to set.
    if i > 0 { v(gap * 2, weak: false) }
    block(above: 0pt, below: 0pt, body)
  }
}

#let nonempty(s) = s != none and str(s).trim() != ""

// Join the parts that are actually present, so a missing company or location
// never leaves a dangling separator.
#let join-parts(parts, sep: ", ") = parts.filter(p => nonempty(p)).join(sep)

// Render `body` as a hyperlink when `url` is set, and as plain content otherwise.
#let maybe-link(url, body) = if nonempty(url) { link(url, body) } else { body }

// A row with content pushed to the left and right edges — the shape every entry
// header uses to right-align its date range.
#let row(left-side, right-side) = grid(
  columns: (1fr, auto),
  column-gutter: 0.8em,
  align(left, left-side),
  align(right + top, right-side),
)

// Group consecutive experience entries that share an employer.
//
// FlowCV prints a single "Bosta" heading above three stacked roles but renders a
// one-off role as "Backend Engineer, Wuilt". Detecting runs here — rather than
// grouping by company globally — preserves the user's ordering, so a return to a
// former employer still reads chronologically instead of being folded together.
//
// With `enabled: false` every entry stands alone, which is what the section's
// "group promotions" switch turns off.
#let group-by-company(items, enabled: true) = {
  if not enabled {
    return items.map(it => (company: it.at("company", default: ""), items: (it,)))
  }
  let groups = ()
  for it in items {
    let company = it.at("company", default: "")
    if groups.len() > 0 and groups.last().company == company and nonempty(company) {
      groups.last().items.push(it)
    } else {
      groups.push((company: company, items: (it,)))
    }
  }
  groups
}

// The work-experience switches, with the defaults a document may omit.
#let experience-options(section) = (
  order: section.at("order", default: "roleFirst"),
  group-promotions: section.at("groupPromotions", default: true),
)

// One entry on a single line: whichever of role and company leads is bold, and
// the company keeps its link wherever it lands.
#let entry-line(it, order, link-company) = {
  let role = it.at("role", default: "")
  let company = it.at("company", default: "")
  let url = it.at("companyUrl", default: "")
  if not nonempty(company) { return text(weight: 700, role) }
  if not nonempty(role) { return link-company(url, text(weight: 700, company)) }

  if order == "companyFirst" {
    link-company(url, text(weight: 700, company)) + text(weight: 700, ", ") + text(weight: "regular", role)
  } else {
    text(weight: 700, role + ", ") + link-company(url, text(weight: "regular", company))
  }
}

// Section dispatch: look up a section's entries whatever the key is called,
// dropping the ones the user has hidden.
//
// Filtering here rather than in each template means hidden entries disappear
// everywhere at once — including from the grouping and "is this section empty?"
// decisions that run off this list.
#let section-items(section) = {
  let raw = if section.kind == "skills" {
    section.at("groups", default: ())
  } else {
    section.at("items", default: ())
  }
  raw.filter(it => it.at("visible", default: true) != false)
}

// ---------------------------------------------------------------------------
// Section bodies.
//
// All three templates lay entries out the same way and differ in typography and
// spacing, so the structure lives here once and each template passes in a
// `style` dict of the pieces it wants to look different:
//
//   heading(title)   a section heading
//   title(body)      an entry's headline (a role, a degree, a project name)
//   company(body)    an employer heading above a run of roles
//   meta(body)       dates and other secondary metadata
//   leading          one line step, for gaps inside an entry
//   line-height      the theme's multiplier, so breaks between entries scale too
//   tight            multiplier on the vertical rhythm between entries
// ---------------------------------------------------------------------------

#let experience-section(items, style, options: (order: "roleFirst", group-promotions: true)) = {
  // Breaks between entries are multiplied by the line height as well as the
  // template's density, so the hierarchy holds at any setting instead of
  // entries closing up as the text inside them spreads.
  let t = style.tight * style.line-height
  for group in group-by-company(items, enabled: options.group-promotions) {
    // A run of roles at one employer gets a single company heading; a lone role
    // reads better inline as "Backend Engineer, Wuilt".
    let grouped = group.items.len() > 1
    if grouped {
      // One line step under the employer — a fixed value here left the first
      // role clamped to the company name at high line heights.
      block(above: 0.95em * t, below: style.leading, (style.company)(maybe-link(
        group.items.first().at("companyUrl", default: ""),
        group.company,
      )))
    }
    for (i, it) in group.items.enumerate() {
      let heading = if grouped {
        (style.title)(it.role)
      } else {
        (style.title)(entry-line(it, options.order, maybe-link))
      }
      block(
        // Roles after the first in a group need a clear break from the previous
        // role's bullets; the first sits tight under its company heading.
        above: if not grouped {
          0.95em * t
        } else if i == 0 {
          style.leading
        } else {
          0.85em * t
        },
        // Same rhythm as the bullets below it, so the line-height control
        // moves this gap too rather than leaving the first bullet stranded.
        below: style.leading,
        row(heading, (style.meta)(fmt-range(
          it.start,
          it.end,
          current: it.at("current", default: false),
        ))),
      )
      if nonempty(it.at("location", default: "")) {
        block(spacing: 0.3em * t, (style.meta)(it.location))
      }
      // Highlights are a rich value like any other description: usually a
      // bulleted list, but paragraphs and a numbered list render here too.
      rich(it.at("bullets", default: ()), gap: style.leading)
    }
  }
}

#let education-section(items, style) = {
  let t = style.tight
  for it in items {
    block(above: 0.85em * t, below: 0.35em * t, row(
      (style.title)(join-parts((it.degree, it.institution))),
      (style.meta)(fmt-range(it.start, it.end, current: it.at("current", default: false))),
    ))
    if rich-nonempty(it.at("description", default: "")) {
      block(spacing: 0.35em * t, rich(it.description, gap: style.leading))
    }
  }
}

#let skills-section(groups, style) = {
  let t = style.tight
  for g in groups {
    let values = g.at("items", default: ()).filter(s => nonempty(s))
    let name = g.at("name", default: "")
    if values.len() == 0 and not nonempty(name) { continue }
    block(above: 0.5em * t, below: 0.5em * t, {
      if nonempty(name) { text(weight: 600, name + ": ") }
      values.join(", ")
    })
  }
}

#let projects-section(items, style) = {
  let t = style.tight
  for it in items {
    let description = it.at("description", default: "")
    // A one-line description reads on the name line; one carrying a list or a
    // second paragraph has nowhere to sit there, so it drops below it.
    let inline-description = rich-inlineable(description)
    block(above: 0.6em * t, below: 0.6em * t, {
      maybe-link(it.at("url", default: ""), (style.title)(it.name))
      if inline-description and rich-nonempty(description) { [, ] + rich-inline(description) }
      let tech = it.at("tech", default: ()).filter(x => nonempty(x))
      if tech.len() > 0 { [ ] + (style.meta)(tech.join(" · ")) }
      if not inline-description {
        block(above: style.leading, below: 0pt, rich(description, gap: style.leading))
      }
    })
  }
}

#let certifications-section(items, style) = {
  let t = style.tight
  for it in items {
    block(above: 0.55em * t, below: 0.2em * t, row(
      maybe-link(it.at("url", default: ""), (style.title)(it.name)),
      (style.meta)(fmt-date(it.at("date", default: none))),
    ))
    if nonempty(it.at("issuer", default: "")) {
      block(spacing: 0.3em * t, (style.meta)(it.issuer))
    }
  }
}

#let languages-section(items, ..) = {
  let parts = items.filter(it => nonempty(it.name)).map(it => {
    if nonempty(it.at("level", default: "")) { it.name + " (" + it.level + ")" } else { it.name }
  })
  if parts.len() > 0 { parts.join(", ") }
}

#let interests-section(items, ..) = {
  let parts = items.filter(it => nonempty(it.name)).map(it => it.name)
  if parts.len() > 0 { parts.join(", ") }
}

#let references-section(items, style) = {
  let t = style.tight
  for it in items {
    block(above: 0.6em * t, below: 0.6em * t, {
      maybe-link(it.at("url", default: ""), (style.title)(it.name))
      let rest = join-parts((it.at("title", default: ""), it.at("company", default: "")))
      if nonempty(rest) { [, ] + rest }
      let contact = join-parts((it.at("email", default: ""), it.at("phone", default: "")))
      if nonempty(contact) { [ ] + (style.meta)(contact) }
    })
  }
}

// Walk the document in the order the user arranged it. Hidden or empty sections
// leave no trace — not even their heading.
#let render-sections(cv, style) = {
  for section in cv.sections {
    let items = section-items(section)
    if not section.at("visible", default: true) or items.len() == 0 { continue }

    (style.heading)(section.title)

    if section.kind == "experience" {
      experience-section(items, style, options: experience-options(section))
    }
    else if section.kind == "education" { education-section(items, style) }
    else if section.kind == "skills" { skills-section(items, style) }
    else if section.kind == "projects" { projects-section(items, style) }
    else if section.kind == "certifications" { certifications-section(items, style) }
    else if section.kind == "languages" { languages-section(items, style) }
    else if section.kind == "interests" { interests-section(items, style) }
    else if section.kind == "references" { references-section(items, style) }
  }
}

// The contact line shared by every template: named links render as their label
// ("GitHub"), raw details as themselves, and anything blank drops out.
#let contact-bits(basics) = {
  let bits = ()
  if nonempty(basics.location) { bits.push(text(basics.location)) }
  if nonempty(basics.email) { bits.push(link("mailto:" + basics.email, basics.email)) }
  if nonempty(basics.phone) { bits.push(link("tel:" + basics.phone, basics.phone)) }
  for l in basics.at("links", default: ()) {
    if nonempty(l.url) {
      bits.push(link(l.url, if nonempty(l.label) { l.label } else { l.url }))
    }
  }
  bits
}
