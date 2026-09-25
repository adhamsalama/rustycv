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

// Rich text arrives either as a bare string (unstyled) or as an array of runs.
// Both are handled here so no template has to know which form it got.
#let rich(value) = {
  if value == none { return [] }
  if type(value) == str { return [#value] }
  if type(value) != array { return [] }

  let out = []
  for run in value {
    let text = run.at("text", default: "")
    if text == "" { continue }
    let piece = [#text]
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

// The plain text behind rich text, for emptiness checks.
#let rich-text(value) = {
  if value == none { return "" }
  if type(value) == str { return value }
  if type(value) != array { return "" }
  value.map(run => run.at("text", default: "")).join("")
}

#let rich-nonempty(value) = rich-text(value).trim() != ""

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

// `gap` is the space between items; pass the paragraph leading so a list
// reads as one evenly-leaded block rather than clumping as line height grows.
#let bullets(items, marker: [•], indent: 0pt, gap: 0.45em) = {
  let items = items.filter(b => rich-nonempty(b))
  if items.len() == 0 { return }
  set list(marker: marker, indent: indent, body-indent: 0.45em, spacing: gap)
  list(..items.map(b => rich(b)))
}

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
//   tight            multiplier on the vertical rhythm between entries
// ---------------------------------------------------------------------------

#let experience-section(items, style, options: (order: "roleFirst", group-promotions: true)) = {
  let t = style.tight
  for group in group-by-company(items, enabled: options.group-promotions) {
    // A run of roles at one employer gets a single company heading; a lone role
    // reads better inline as "Backend Engineer, Wuilt".
    let grouped = group.items.len() > 1
    if grouped {
      block(above: 0.95em * t, below: 0.4em * t, (style.company)(maybe-link(
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
        above: if not grouped { 0.95em * t } else if i == 0 { 0.4em * t } else { 0.85em * t },
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
      bullets(it.at("bullets", default: ()), gap: style.leading)
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
      block(spacing: 0.35em * t, rich(it.description))
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
    block(above: 0.6em * t, below: 0.6em * t, {
      maybe-link(it.at("url", default: ""), (style.title)(it.name))
      if rich-nonempty(it.at("description", default: "")) { [, ] + rich(it.description) }
      let tech = it.at("tech", default: ()).filter(x => nonempty(x))
      if tech.len() > 0 { [ ] + (style.meta)(tech.join(" · ")) }
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
