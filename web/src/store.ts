import { create } from 'zustand'
import { immer } from 'zustand/middleware/immer'
import type { BaseItem, CvDocument, Section, SectionKind, Theme } from './types'
import { sectionItems, setSectionItems } from './types'
import { blankItem, defaultTitle } from './sections'

/**
 * The document being edited.
 *
 * Server state (the list of CVs, templates) lives in React Query; this holds
 * only the in-flight document, because it changes on every keystroke and does
 * not want cache semantics.
 */
interface CvStore {
  id: string | null
  title: string
  document: CvDocument | null
  /** Bumped on every edit; the autosave and preview effects watch it. */
  revision: number

  load: (id: string, title: string, document: CvDocument) => void
  reset: () => void

  setTitle: (title: string) => void
  setTemplate: (template: string) => void
  patchTheme: (patch: Partial<Theme>) => void
  patchBasics: (patch: Partial<CvDocument['basics']>) => void

  addLink: () => void
  updateLink: (id: string, patch: { label?: string; url?: string }) => void
  removeLink: (id: string) => void

  addSection: (kind: SectionKind) => void
  removeSection: (sectionId: string) => void
  renameSection: (sectionId: string, title: string) => void
  toggleSection: (sectionId: string) => void
  moveSection: (from: number, to: number) => void

  addItem: (sectionId: string) => void
  updateItem: (sectionId: string, itemId: string, patch: Record<string, unknown>) => void
  toggleItem: (sectionId: string, itemId: string) => void
  removeItem: (sectionId: string, itemId: string) => void
  moveItem: (sectionId: string, from: number, to: number) => void
}

function move<T>(list: T[], from: number, to: number): void {
  if (from === to || from < 0 || to < 0 || from >= list.length || to >= list.length) return
  const [item] = list.splice(from, 1)
  if (item !== undefined) list.splice(to, 0, item)
}

export const useCvStore = create<CvStore>()(
  immer((set) => {
    // Every mutation runs through here so `revision` can never be forgotten —
    // a missed bump would silently stop autosave and the preview.
    const edit = (recipe: (document: CvDocument, state: CvStore) => void) =>
      set((state) => {
        if (!state.document) return
        recipe(state.document, state)
        state.revision += 1
      })

    const withSection = (sectionId: string, recipe: (section: Section) => void) =>
      edit((document) => {
        const section = document.sections.find((s) => s.id === sectionId)
        if (section) recipe(section)
      })

    return {
      id: null,
      title: '',
      document: null,
      revision: 0,

      load: (id, title, document) =>
        set((state) => {
          state.id = id
          state.title = title
          state.document = document
          state.revision = 0
        }),

      reset: () =>
        set((state) => {
          state.id = null
          state.title = ''
          state.document = null
          state.revision = 0
        }),

      setTitle: (title) =>
        set((state) => {
          state.title = title
          state.revision += 1
        }),

      setTemplate: (template) => edit((document) => void (document.template = template)),
      patchTheme: (patch) => edit((document) => void Object.assign(document.theme, patch)),
      patchBasics: (patch) => edit((document) => void Object.assign(document.basics, patch)),

      addLink: () =>
        edit((document) => {
          document.basics.links.push({ id: crypto.randomUUID(), label: '', url: '' })
        }),
      updateLink: (id, patch) =>
        edit((document) => {
          const link = document.basics.links.find((l) => l.id === id)
          if (link) Object.assign(link, patch)
        }),
      removeLink: (id) =>
        edit((document) => {
          document.basics.links = document.basics.links.filter((l) => l.id !== id)
        }),

      addSection: (kind) =>
        edit((document) => {
          const section = {
            id: crypto.randomUUID(),
            title: defaultTitle(kind),
            visible: true,
            ...(kind === 'skills' ? { kind, groups: [] } : { kind, items: [] }),
          } as Section
          document.sections.push(section)
        }),
      removeSection: (sectionId) =>
        edit((document) => {
          document.sections = document.sections.filter((s) => s.id !== sectionId)
        }),
      renameSection: (sectionId, title) => withSection(sectionId, (s) => void (s.title = title)),
      toggleSection: (sectionId) => withSection(sectionId, (s) => void (s.visible = !s.visible)),
      moveSection: (from, to) => edit((document) => move(document.sections, from, to)),

      addItem: (sectionId) =>
        withSection(sectionId, (section) => {
          const items = sectionItems(section)
          items.push(blankItem(section.kind))
          setSectionItems(section, items)
        }),
      updateItem: (sectionId, itemId, patch) =>
        withSection(sectionId, (section) => {
          const item = sectionItems(section).find((i: BaseItem) => i.id === itemId)
          if (item) Object.assign(item, patch)
        }),
      toggleItem: (sectionId, itemId) =>
        withSection(sectionId, (section) => {
          const item = sectionItems(section).find((i: BaseItem) => i.id === itemId)
          if (item) item.visible = item.visible === false
        }),
      removeItem: (sectionId, itemId) =>
        withSection(sectionId, (section) => {
          setSectionItems(
            section,
            sectionItems(section).filter((i: BaseItem) => i.id !== itemId),
          )
        }),
      moveItem: (sectionId, from, to) =>
        withSection(sectionId, (section) => move(sectionItems(section), from, to)),
    }
  }),
)
