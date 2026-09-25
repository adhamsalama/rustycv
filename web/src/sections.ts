import type { BaseItem, SectionKind } from './types'

/**
 * How to edit one field of an item.
 *
 * Every section kind is described by data rather than by its own component:
 * eight near-identical forms would be eight places to fix the next bug, and the
 * shapes genuinely only differ in which fields they have.
 */
export type FieldSpec =
  | { key: string; label: string; type: 'text' | 'url' | 'email' | 'tel'; placeholder?: string; width?: 'full' | 'half' }
  | { key: string; label: string; type: 'textarea'; placeholder?: string; rows?: number }
  | { key: string; label: string; type: 'date'; width?: 'half' }
  | { key: string; label: string; type: 'checkbox' }
  /** A `string[]` edited one entry per line. */
  | { key: string; label: string; type: 'lines'; placeholder?: string; rows?: number }
  /** A `string[]` edited as a comma-separated list. */
  | { key: string; label: string; type: 'tags'; placeholder?: string }

export interface SectionSpec {
  label: string
  /** Word for one entry, used on the "Add …" button. */
  itemLabel: string
  defaultTitle: string
  fields: FieldSpec[]
  /** One-line label for the collapsed row in the editor. */
  summary: (item: BaseItem) => string
}

const str = (item: BaseItem, key: string): string => {
  const value = item[key]
  return typeof value === 'string' ? value : ''
}

export const SECTION_SPECS: Record<SectionKind, SectionSpec> = {
  experience: {
    label: 'Work Experience',
    itemLabel: 'role',
    defaultTitle: 'Work Experience',
    fields: [
      { key: 'role', label: 'Role', type: 'text', width: 'half', placeholder: 'Staff Backend Engineer' },
      { key: 'company', label: 'Company', type: 'text', width: 'half', placeholder: 'Bosta' },
      { key: 'companyUrl', label: 'Company link', type: 'url', width: 'half' },
      { key: 'location', label: 'Location', type: 'text', width: 'half' },
      { key: 'start', label: 'Start', type: 'date', width: 'half' },
      { key: 'end', label: 'End', type: 'date', width: 'half' },
      { key: 'current', label: 'I currently work here', type: 'checkbox' },
      { key: 'bullets', label: 'Highlights', type: 'lines', rows: 6, placeholder: 'One achievement per line' },
    ],
    summary: (item) => [str(item, 'role'), str(item, 'company')].filter(Boolean).join(' · ') || 'New role',
  },
  education: {
    label: 'Education',
    itemLabel: 'qualification',
    defaultTitle: 'Education',
    fields: [
      { key: 'degree', label: 'Degree', type: 'text', width: 'half', placeholder: 'Computer Engineering' },
      { key: 'institution', label: 'Institution', type: 'text', width: 'half' },
      { key: 'location', label: 'Location', type: 'text', width: 'half' },
      { key: 'start', label: 'Start', type: 'date', width: 'half' },
      { key: 'end', label: 'End', type: 'date', width: 'half' },
      { key: 'current', label: 'Currently studying', type: 'checkbox' },
      { key: 'description', label: 'Notes', type: 'textarea', rows: 2 },
    ],
    summary: (item) =>
      [str(item, 'degree'), str(item, 'institution')].filter(Boolean).join(', ') || 'New qualification',
  },
  skills: {
    label: 'Skills',
    itemLabel: 'group',
    defaultTitle: 'Skills',
    fields: [
      { key: 'name', label: 'Category', type: 'text', placeholder: 'Languages' },
      { key: 'items', label: 'Skills', type: 'tags', placeholder: 'Go, Rust, TypeScript' },
    ],
    summary: (item) => str(item, 'name') || 'New group',
  },
  projects: {
    label: 'Projects',
    itemLabel: 'project',
    defaultTitle: 'Projects',
    fields: [
      { key: 'name', label: 'Name', type: 'text', width: 'half' },
      { key: 'url', label: 'Link', type: 'url', width: 'half' },
      { key: 'description', label: 'Description', type: 'textarea', rows: 3 },
      { key: 'tech', label: 'Tech', type: 'tags', placeholder: 'Go, Kubernetes, React' },
    ],
    summary: (item) => str(item, 'name') || 'New project',
  },
  certifications: {
    label: 'Certifications',
    itemLabel: 'certification',
    defaultTitle: 'Certifications',
    fields: [
      { key: 'name', label: 'Name', type: 'text', width: 'half' },
      { key: 'issuer', label: 'Issuer', type: 'text', width: 'half' },
      { key: 'url', label: 'Link', type: 'url', width: 'half' },
      { key: 'date', label: 'Date', type: 'date', width: 'half' },
    ],
    summary: (item) => str(item, 'name') || 'New certification',
  },
  languages: {
    label: 'Languages',
    itemLabel: 'language',
    defaultTitle: 'Languages',
    fields: [
      { key: 'name', label: 'Language', type: 'text', width: 'half' },
      { key: 'level', label: 'Level', type: 'text', width: 'half', placeholder: 'Native' },
    ],
    summary: (item) => str(item, 'name') || 'New language',
  },
  interests: {
    label: 'Interests',
    itemLabel: 'interest',
    defaultTitle: 'Interests',
    fields: [{ key: 'name', label: 'Interest', type: 'text' }],
    summary: (item) => str(item, 'name') || 'New interest',
  },
  references: {
    label: 'References',
    itemLabel: 'reference',
    defaultTitle: 'References',
    fields: [
      { key: 'name', label: 'Name', type: 'text', width: 'half' },
      { key: 'url', label: 'Link', type: 'url', width: 'half' },
      { key: 'title', label: 'Title', type: 'text', width: 'half' },
      { key: 'company', label: 'Company', type: 'text', width: 'half' },
      { key: 'email', label: 'Email', type: 'email', width: 'half' },
      { key: 'phone', label: 'Phone', type: 'tel', width: 'half' },
    ],
    summary: (item) => str(item, 'name') || 'New reference',
  },
}

export const SECTION_KINDS = Object.keys(SECTION_SPECS) as SectionKind[]

export const defaultTitle = (kind: SectionKind): string => SECTION_SPECS[kind].defaultTitle

/** A new entry with every field at its empty value, matching the Rust defaults. */
export function blankItem(kind: SectionKind): BaseItem {
  const item: BaseItem = { id: crypto.randomUUID(), visible: true }
  for (const field of SECTION_SPECS[kind].fields) {
    switch (field.type) {
      case 'checkbox':
        item[field.key] = false
        break
      case 'date':
        item[field.key] = null
        break
      case 'lines':
      case 'tags':
        item[field.key] = []
        break
      default:
        item[field.key] = ''
    }
  }
  return item
}
