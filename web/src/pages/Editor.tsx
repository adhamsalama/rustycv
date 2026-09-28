import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Link, useParams, useSearchParams } from 'react-router-dom'
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
import { PdfPreview, type PdfPreviewHandle } from '../components/PdfPreview'
import { warmRasteriser } from '../components/PdfDocument'
import { AppearanceToggle } from '../components/AppearanceToggle'
import { DownloadPdf } from '../components/DownloadPdf'
import { ShareControl } from '../components/ShareControl'
import { SiteFooter } from '../components/SiteFooter'
import { SortableList, SortableRow } from '../components/Sortable'
import { GripIcon, Icon } from '../components/Icon'

type Pane = { kind: 'details' } | { kind: 'design' } | { kind: 'section'; id: string }

/**
 * Which pane is open, kept in the URL rather than in component state so a
 * refresh comes back to the section you were editing instead of to Details.
 *
 * One parameter carries all three cases: a section's id is a UUID, so it can
 * never collide with the two literals.
 */
function usePane(): [Pane, (pane: Pane) => void] {
  const [params, setParams] = useSearchParams()
  const value = params.get('pane') ?? 'details'

  const pane: Pane =
    value === 'details' || value === 'design' ? { kind: value } : { kind: 'section', id: value }

  const setPane = useCallback(
    (next: Pane) => {
      setParams(
        (previous) => {
          const updated = new URLSearchParams(previous)
          // Details is the default, so it stays out of the URL entirely.
          if (next.kind === 'details') updated.delete('pane')
          else updated.set('pane', next.kind === 'design' ? 'design' : next.id)
          return updated
        },
        // Replace rather than push: switching panes is not a navigation, and
        // pushing would make Back walk every section you had opened before it
        // finally left the editor.
        { replace: true },
      )
    },
    [setParams],
  )

  return [pane, setPane]
}

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

  // Where this instance renders. Decided by whoever started the server and
  // fixed for the life of its process, so it is read once and never refetched
  // — and the editor waits for it below rather than guessing, because guessing
  // wrong means a first render on the machine the operator ruled out.
  const { data: config } = useQuery({
    queryKey: ['config'],
    queryFn: api.config,
    staleTime: Infinity,
  })

  // Start fetching and compiling the PDF rasteriser now, so it is not still
  // arriving when the first render comes back.
  useEffect(() => {
    warmRasteriser()
  }, [])

  useEffect(() => {
    if (data) store.load(data.id, data.title, data.document)
    return () => store.reset()
    // Reload only when a different CV is opened, not on every store change.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [data?.id])

  const [pane, setPane] = usePane()
  const { state: saveState, renderAt } = useAutosave(id, revision)
  // The outline becomes a slide-in drawer once it no longer fits beside the
  // pane; closed by default so a phone doesn't open on top of it.
  const [outlineOpen, setOutlineOpen] = useState(false)
  const previewRef = useRef<PdfPreviewHandle>(null)

  if (isLoading) return <main className="centered">Loading…</main>
  if (error) return <main className="centered">Could not load this CV.</main>
  if (!document || !config) return <main className="centered">Loading…</main>

  const activeSection =
    pane.kind === 'section' ? document.sections.find((s) => s.id === pane.id) : undefined

  // Choosing anything from the drawer is the signal that its job is done.
  const choose = (next: Pane) => {
    setPane(next)
    setOutlineOpen(false)
  }

  return (
    <>
      <div className="editor">
        <header className="topbar">
        <button
          type="button"
          className="ghost outline-toggle"
          aria-label="Show sections"
          aria-expanded={outlineOpen}
          onClick={() => setOutlineOpen((o) => !o)}
        >
          <Icon name="menu" />
        </button>
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
        {/* The stacked preview is already on the page below the fields, but a
            small screen means scrolling past all of them to reach it — this
            jumps straight to the same overlay the expand icon on the preview
            itself opens. */}
        <button
          type="button"
          className="ghost preview-toggle"
          onClick={() => previewRef.current?.expand()}
        >
          Preview
        </button>
        <AppearanceToggle />
        {data ? <ShareControl cv={data} /> : null}
        <div className="editor-actions">
          <a className="ghost" href={api.exportUrl(id)}>
            Export JSON
          </a>
          <DownloadPdf id={id} document={document} title={title} mode={config.renderMode} />
        </div>
      </header>

      <div className="editor-body">
        {outlineOpen ? (
          <div className="outline-scrim" onClick={() => setOutlineOpen(false)} />
        ) : null}
        <nav className={outlineOpen ? 'outline open' : 'outline'}>
          <button
            type="button"
            className={pane.kind === 'details' ? 'outline-item active' : 'outline-item'}
            onClick={() => choose({ kind: 'details' })}
          >
            Details
          </button>
          <button
            type="button"
            className={pane.kind === 'design' ? 'outline-item active' : 'outline-item'}
            onClick={() => choose({ kind: 'design' })}
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
                        <GripIcon />
                      </button>
                      <button
                        type="button"
                        className={
                          pane.kind === 'section' && pane.id === section.id
                            ? 'outline-item active'
                            : 'outline-item'
                        }
                        onClick={() => choose({ kind: 'section', id: section.id })}
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
              if (added) choose({ kind: 'section', id: added.id })
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
          <PdfPreview
            ref={previewRef}
            document={document}
            renderAt={renderAt}
            mode={config.renderMode}
          />
        </aside>
      </div>
      </div>

      {/* Below the editor rather than inside it. The shell above is exactly
          one viewport tall, so this follows it just past the fold: it spans
          the outline, the fields and the preview together, and you reach it
          by scrolling to the end of the page — instead of it taking a
          permanent strip out of the one screen where vertical space is the
          scarce thing. */}
      <footer className="editor-credit">
        <SiteFooter />
      </footer>
    </>
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
  // legal but is almost never what someone means. Custom sections are the
  // exception — there's no fixed number of them, so it always stays offered.
  const available = useMemo(
    () => SECTION_KINDS.filter((kind) => kind === 'custom' || !existing.includes(kind)),
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
