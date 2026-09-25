// Mirrors `rustycv-core`. Keep in sync with crates/rustycv-core/src.

export type SectionKind =
  | 'experience'
  | 'education'
  | 'skills'
  | 'projects'
  | 'certifications'
  | 'languages'
  | 'interests'
  | 'references'

export interface DateSpec {
  year: number
  month?: number | null
}

export type PageSize = 'a4' | 'letter'
export type HeadingStyle = 'bold' | 'underline' | 'caps'

export interface Theme {
  accent: string
  fontFamily: string
  fontSizePt: number
  page: PageSize
  marginMm: number
  lineHeight: number
  sectionGapMm: number
  headingStyle: HeadingStyle
}

export interface Link {
  id: string
  label: string
  url: string
}

export interface Basics {
  fullName: string
  headline: string
  email: string
  phone: string
  location: string
  links: Link[]
  summary: string
}

/** Every item shares an id; the rest varies by section kind. */
export interface BaseItem {
  id: string
  [key: string]: unknown
}

export interface ExperienceItem extends BaseItem {
  role: string
  company: string
  companyUrl: string
  location: string
  start: DateSpec | null
  end: DateSpec | null
  current: boolean
  bullets: string[]
}

export interface SkillGroup extends BaseItem {
  name: string
  items: string[]
}

/**
 * A section is flat on the wire: common fields plus a `kind` tag and the
 * entries, which live under `items` for every kind except skills (`groups`).
 */
export type Section = {
  id: string
  title: string
  visible: boolean
} & (
  | { kind: 'skills'; groups: SkillGroup[] }
  | { kind: Exclude<SectionKind, 'skills'>; items: BaseItem[] }
)

export interface CvDocument {
  schemaVersion: number
  template: string
  theme: Theme
  basics: Basics
  sections: Section[]
}

export interface CvSummary {
  id: string
  title: string
  template: string
  fullName: string
  createdAt: string
  updatedAt: string
}

export interface Cv {
  id: string
  title: string
  createdAt: string
  updatedAt: string
  document: CvDocument
}

export interface TemplateInfo {
  id: string
  name: string
  description: string
}

export interface Diagnostic {
  severity: string
  message: string
  file: string | null
  line: number | null
  hints: string[]
}

/** The entries of a section, whatever the key happens to be called. */
export function sectionItems(section: Section): BaseItem[] {
  return section.kind === 'skills' ? section.groups : section.items
}

export function setSectionItems(section: Section, items: BaseItem[]): void {
  if (section.kind === 'skills') {
    section.groups = items as SkillGroup[]
  } else {
    section.items = items
  }
}
