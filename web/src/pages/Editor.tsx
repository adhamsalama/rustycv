import { useEffect, useMemo, useRef, useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import { useCvStore } from '../store'
import { SECTION_KINDS, SECTION_SPECS } from '../sections'
import type { SectionKind } from '../types'
import { sectionItems, visibleItems } from '../types'
import { useDebounced } from '../hooks/useDebounced'
import { BasicsEditor } from '../components/BasicsEditor'
import { SectionEditor } from '../components/SectionEditor'
import { ThemePanel } from '../components/ThemePanel'
import { PdfPreview } from '../components/PdfPreview'
import { SortableList, SortableRow } from '../components/Sortable'

type Pane = { kind: 'details' } | { kind: 'design' } | { kind: 'section'; id: string }

/**
 * How long the editor must be idle before the document is worth a round trip.
 *
 * One delay now, not two: the save and the render are a single chain, so there
 * is nothing left to tune them against each other.
 */
const EDIT_SETTLE_MS = 400

export function Editor() {
  const { id = '' } = useParams()
  const store = useCvStore()
  const { document, revision, title } = store

  const { data, isLoading, error } = useQuery({
    queryKey: ['cv', id],
    queryFn: () => api.getCv(id),
    enabled: Boolean(id),
  })

  useEffect(() => {
    if (data) store.load(data.id, data.title, data.document)
    return () => store.reset()
    // Reload only when a different CV is opened, not on every store change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.id])

  const [pane, setPane] = useState<Pane>({ kind: 'details' })
  const { state: saveState, renderAt } = useAutosave(id, revision)

  if (isLoading) return <main className="centered">Loading…</main>
  if (error) return <main className="centered">Could not load this CV.</main>
  if (!document) return <main className="centered">Loading…</main>

  const activeSection =
    pane.kind === 'section' ? document.sections.find((s) => s.id === pane.id) : undefined

  return (
    <div className="editor">
      <header className="topbar">
        <Link to="/" className="ghost">
          ← All CVs
        </Link>
        <input
          className="cv-title"
          value={title}
          onChange={(e) => store.setTitle(e.target.value)}
          aria-label="CV name"
        />
        <span className={`save-state save-${saveState}`}>
          {saveState === 'saving' ? 'Saving…' : saveState === 'error' ? 'Save failed' : 'Saved'}
        </span>
        <div className="spacer" />
        <a className="ghost" href={api.exportUrl(id)}>
          Export JSON
        </a>
        <a className="primary" href={api.pdfUrl(id)}>
          Download PDF
        </a>
      </header>

      <div className="editor-body">
        <nav className="outline">
          <button
            type="button"
            className={pane.kind === 'details' ? 'outline-item active' : 'outline-item'}
            onClick={() => setPane({ kind: 'details' })}
          >
            Details
          </button>
          <button
            type="button"
            className={pane.kind === 'design' ? 'outline-item active' : 'outline-item'}
            onClick={() => setPane({ kind: 'design' })}
          >
            Design
          </button>

          <h3 className="outline-head">Sections</h3>
          <SortableList
            ids={document.sections.map((s) => s.id)}
            onReorder={store.moveSection}
          >
            <div>
              {document.sections.map((section) => (
                <SortableRow key={section.id} id={section.id}>
                  {(handleProps) => (
                    <div className="outline-row">
                      <button
                        type="button"
                        className="drag-handle"
                        aria-label={`Reorder ${section.title}`}
                        {...handleProps}
                      >
                        ⠿
                      </button>
                      <button
                        type="button"
                        className={
                          pane.kind === 'section' && pane.id === section.id
                            ? 'outline-item active'
                            : 'outline-item'
                        }
                        onClick={() => setPane({ kind: 'section', id: section.id })}
                      >
                        <span className={section.visible ? '' : 'muted'}>{section.title}</span>
                        <span className="count">
                          {visibleItems(section).length < sectionItems(section).length
                            ? `${visibleItems(section).length}/${sectionItems(section).length}`
                            : sectionItems(section).length}
                        </span>
                      </button>
                    </div>
                  )}
                </SortableRow>
              ))}
            </div>
          </SortableList>

          <AddSection
            existing={document.sections.map((s) => s.kind)}
            onAdd={(kind) => {
              store.addSection(kind)
              const added = useCvStore.getState().document?.sections.at(-1)
              if (added) setPane({ kind: 'section', id: added.id })
            }}
          />
        </nav>

        <main className="pane">
          {pane.kind === 'details' ? <BasicsEditor /> : null}
          {pane.kind === 'design' ? <ThemePanel /> : null}
          {activeSection ? <SectionEditor key={activeSection.id} section={activeSection} /> : null}
          {pane.kind === 'section' && !activeSection ? (
            <p className="muted">That section was deleted.</p>
          ) : null}
        </main>

        <aside className="preview-pane">
          <PdfPreview document={document} renderAt={renderAt} />
        </aside>
      </div>
    </div>
  )
}

function AddSection({
  existing,
  onAdd,
}: {
  existing: SectionKind[]
  onAdd: (kind: SectionKind) => void
}) {
  // Offer the kinds that aren't in the CV yet first; a second Skills section is
  // legal but is almost never what someone means.
  const available = useMemo(
    () => SECTION_KINDS.filter((kind) => !existing.includes(kind)),
    [existing],
  )

  if (available.length === 0) return null

  return (
    <label className="field add-section">
      <span className="field-label">Add section</span>
      <select
        value=""
        onChange={(e) => {
          if (e.target.value) onAdd(e.target.value as SectionKind)
        }}
      >
        <option value="">Choose…</option>
        {available.map((kind) => (
          <option key={kind} value={kind}>
            {SECTION_SPECS[kind].label}
          </option>
        ))}
      </select>
    </label>
  )
}

type SaveState = 'saved' | 'saving' | 'error'

/**
 * Persist the document once the editor settles, then let the preview render.
 *
 * The write leads deliberately: the preview is the artefact people trust, so
 * it must never be ahead of what the server holds. The returned `renderAt`
 * token is that permission — it changes only after a save has come back, and
 * the preview renders when it does.
 *
 * A *failed* save releases the token too. A preview frozen because the
 * database is unhappy is worse than one showing work that isn't persisted yet,
 * and the header already says the save failed.
 */
function useAutosave(id: string, revision: number): { state: SaveState; renderAt: string } {
  const settled = useDebounced(revision, EDIT_SETTLE_MS)
  const [state, setState] = useState<SaveState>('saved')
  const [renderedRevision, setRenderedRevision] = useState(0)
  const lastAttempt = useRef(0)

  // A different CV has been opened: nothing has been written for it yet, and
  // its revisions count from zero again, so a stale high-water mark here would
  // swallow its first save.
  useEffect(() => {
    lastAttempt.current = 0
    setRenderedRevision(0)
  }, [id])

  useEffect(() => {
    // revision 0 is the freshly-loaded document — nothing to write back. The
    // preview still renders it, on the `${id}:0` token below.
    if (settled === 0 || settled === lastAttempt.current) return

    const { id: storeId, document, title } = useCvStore.getState()
    if (!document || storeId !== id) return

    lastAttempt.current = settled
    let cancelled = false
    setState('saving')

    const release = () => setRenderedRevision(settled)

    api
      .saveCv(id, document, title)
      .then(() => {
        if (cancelled) return
        setState('saved')
        release()
      })
      .catch(() => {
        if (cancelled) return
        setState('error')
        release()
      })

    return () => {
      cancelled = true
    }
  }, [settled, id])

  // Keyed by CV as well as revision, so switching CVs re-renders even though
  // the new document starts back at revision 0.
  return { state, renderAt: `${id}:${renderedRevision}` }
}
