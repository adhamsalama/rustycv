// Mirrors `rustycv-core`. Keep in sync with crates/rustycv-core/src.

import type { RichText } from './rich'
export type { RichText, Run } from './rich'

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
  summary: RichText
}

/** Every item shares an id and a visibility flag; the rest varies by kind. */
export interface BaseItem {
  id: string
  /** Hidden entries are kept in the document but left out of the PDF. */
  visible: boolean
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
  bullets: RichText
}

export interface SkillGroup extends BaseItem {
  name: string
  items: string[]
}

/**
 * A section is flat on the wire: common fields plus a `kind` tag and the
 * entries, which live under `items` for every kind except skills (`groups`).
 */
/** Which of role and company leads a work-experience entry. */
export type EntryOrder = 'roleFirst' | 'companyFirst'

export type Section = {
  id: string
  title: string
  visible: boolean
} & (
  | { kind: 'skills'; groups: SkillGroup[] }
  | {
      kind: 'experience'
      items: BaseItem[]
      order: EntryOrder
      groupPromotions: boolean
    }
  | { kind: Exclude<SectionKind, 'skills' | 'experience'>; items: BaseItem[] }
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

/** The spacing a template is designed around — where "reset design" lands. */
export interface Metrics {
  fontSizePt: number
  marginMm: number
  lineHeight: number
  sectionGapMm: number
}

export interface TemplateInfo {
  id: string
  name: string
  description: string
  metrics: Metrics
}

/** The theme fields the reset control owns. Keep in step with `Metrics`. */
export const METRIC_KEYS = ['fontSizePt', 'marginMm', 'lineHeight', 'sectionGapMm'] as const

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

/** Entries that would actually appear in the PDF. */
export function visibleItems(section: Section): BaseItem[] {
  return sectionItems(section).filter((item) => item.visible !== false)
}

export function setSectionItems(section: Section, items: BaseItem[]): void {
  if (section.kind === 'skills') {
    section.groups = items as SkillGroup[]
  } else {
    section.items = items
  }
}
