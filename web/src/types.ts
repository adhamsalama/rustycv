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
export type BulletStyle = 'dot' | 'dash'

export interface Theme {
  accent: string
  fontFamily: string
  fontSizePt: number
  page: PageSize
  marginMm: number
  lineHeight: number
  sectionGapMm: number
  headingStyle: HeadingStyle
  bulletStyle: BulletStyle
}

/** The signed-in account. Mirrors `auth::User` — never carries the hash. */
export interface User {
  id: string
  email: string
  createdAt: string
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

// ---------------------------------------------------------- job tracker

export type ApplicationStatus = 'wishlist' | 'applied' | 'interview' | 'offer' | 'rejected'

/**
 * The board's columns, in the order they are drawn. A closed set, matching
 * `Status` in `crates/rustycv-server/src/jobs.rs` — the server rejects
 * anything else.
 */
export const STATUS_COLUMNS: { status: ApplicationStatus; label: string }[] = [
  { status: 'wishlist', label: 'Wishlist' },
  { status: 'applied', label: 'Applied' },
  { status: 'interview', label: 'Interview' },
  { status: 'offer', label: 'Offer' },
  { status: 'rejected', label: 'Closed' },
]

export interface Application {
  id: string
  company: string
  role: string
  url: string
  notes: string
  status: ApplicationStatus
  /** The CV this was sent with, if one was. Null once that CV is deleted. */
  cvId: string | null
  cvTitle: string | null
  createdAt: string
  updatedAt: string
}

/** A card's own fields. Where it sits in its column is the server's business. */
export type ApplicationInput = Pick<
  Application,
  'company' | 'role' | 'url' | 'notes' | 'status' | 'cvId'
>

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
