import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from 'react'
import { createPortal } from 'react-dom'
import { ApiError } from '../api'
import { renderPdf } from '../renderer'
import type { CvDocument, Diagnostic, RenderMode } from '../types'

interface PreviewState {
  url: string | null
  pending: boolean
  diagnostics: Diagnostic[]
  error: string | null
  /** Why the render on screen did not happen where it was asked to, if so. */
  fellBackBecause: string | null
}

/** Lets a control outside this component (the topbar's small-screen "Preview"
 * button) open the same full-CV overlay as the ⤢ icon and the "e" shortcut. */
export interface PdfPreviewHandle {
  expand: () => void
}

export const PdfPreview = forwardRef<PdfPreviewHandle, {
  document: CvDocument
  /**
   * Permission to render, from the autosave: a token that changes once the
   * edit has been written back. Debouncing and ordering both happen there, so
   * this component simply renders whenever the token moves.
   */
  renderAt: string
  /** Which machine compiles it, as `/api/config` named. */
  mode: RenderMode
}>(function PdfPreview({ document, renderAt, mode }, ref) {
  const [expanded, setExpanded] = useState(false)
  const close = useCallback(() => setExpanded(false), [])
  useImperativeHandle(ref, () => ({ expand: () => setExpanded(true) }), [])
  const [state, setState] = useState<PreviewState>({
    url: null,
    pending: true,
    diagnostics: [],
    error: null,
    fellBackBecause: null,
  })

  // Read the document without making it an effect dependency — the token is
  // what says a saved change is ready to render.
  const documentRef = useRef(document)
  documentRef.current = document

  useEffect(() => {
    const controller = new AbortController()
    let cancelled = false
    setState((s) => ({ ...s, pending: true }))

    renderPdf(documentRef.current, mode, controller.signal)
      .then(({ blob, fellBackBecause }) => {
        if (cancelled) return
        const url = URL.createObjectURL(blob)
        setState((previous) => {
          // Swap first, then revoke: releasing the old URL before the iframe has
          // the new one makes the preview flash blank on every edit.
          if (previous.url) URL.revokeObjectURL(previous.url)
          return {
            url,
            pending: false,
            diagnostics: [],
            error: null,
            fellBackBecause: fellBackBecause ?? null,
          }
        })
      })
      .catch((error: unknown) => {
        if (cancelled || (error instanceof DOMException && error.name === 'AbortError')) return
        // Keep the last good render on screen and explain what broke over it.
        setState((previous) => ({
          ...previous,
          pending: false,
          diagnostics: error instanceof ApiError ? error.diagnostics : [],
          error: error instanceof Error ? error.message : 'Could not render the preview',
        }))
      })

    return () => {
      cancelled = true
      controller.abort()
    }
  }, [renderAt, mode])

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') return setExpanded(false)
      // Don't steal "e" from whatever field the user is typing in.
      const target = event.target as HTMLElement | null
      const typing =
        target?.isContentEditable ||
        ['INPUT', 'TEXTAREA', 'SELECT'].includes(target?.tagName ?? '')
      if (!typing && event.key.toLowerCase() === 'e' && !event.metaKey && !event.ctrlKey) {
        setExpanded(true)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  // Release the final object URL when the preview unmounts.
  const urlRef = useRef<string | null>(null)
  urlRef.current = state.url
  useEffect(() => () => void (urlRef.current && URL.revokeObjectURL(urlRef.current)), [])

  return (
    <div className="preview">
      <div className="preview-status">
        {state.pending ? <span className="badge">Rendering…</span> : null}
        {state.error ? <span className="badge badge-error">{state.error}</span> : null}
        {/* Silent when the render happened where it was configured to. A
            render that worked is not news; one that quietly moved machines is,
            because it is the only visible sign the wasm module is missing. */}
        {state.fellBackBecause ? (
          <span className="badge badge-notice" title={state.fellBackBecause}>
            Rendered on the server
          </span>
        ) : null}
      </div>

      {state.diagnostics.length > 0 ? (
        <ul className="diagnostics">
          {state.diagnostics.map((d, index) => (
            <li key={index}>
              <code>
                {d.file ?? 'template'}
                {d.line ? `:${d.line}` : ''}
              </code>{' '}
              {d.message}
            </li>
          ))}
        </ul>
      ) : null}

      {state.url ? (
        <>
          {/* The iframe swallows clicks into the PDF viewer, so expanding needs
              its own control rather than a click handler on the preview. */}
          <button
            type="button"
            className="preview-expand"
            onClick={() => setExpanded(true)}
            title="Open the full CV (or press E)"
            aria-label="Open the full CV"
          >
            ⤢
          </button>
          <iframe
            className="preview-frame"
            title="CV preview"
            src={`${state.url}#toolbar=0&view=FitH`}
          />
        </>
      ) : (
        <div className="preview-empty">{state.error ? 'No preview' : 'Rendering your CV…'}</div>
      )}

      {expanded ? <ExpandedPreview url={state.url} onClose={close} /> : null}
    </div>
  )
})

/**
 * The full CV over a dimmed editor.
 *
 * Rendered into `document.body` so no ancestor's stacking context or
 * `overflow` can clip it, and reusing the preview's existing blob URL so
 * opening it costs nothing and it keeps updating as you edit.
 */
function ExpandedPreview({ url, onClose }: { url: string | null; onClose: () => void }) {
  const closeRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    // Focus the close button so Escape and Tab have somewhere sensible to start
    // — without it, focus stays behind the dimmed backdrop.
    closeRef.current?.focus()

    const previousOverflow = window.document.body.style.overflow
    window.document.body.style.overflow = 'hidden'
    return () => {
      window.document.body.style.overflow = previousOverflow
    }
  }, [])

  return createPortal(
    <div
      className="lightbox"
      role="dialog"
      aria-modal="true"
      aria-label="Full CV preview"
      onMouseDown={(e) => {
        // Only a click on the backdrop itself closes — not one that started
        // inside the document and drifted out while selecting.
        if (e.target === e.currentTarget) onClose()
      }}
    >
      <button
        ref={closeRef}
        type="button"
        className="lightbox-close"
        onClick={onClose}
        aria-label="Close preview"
      >
        ✕
      </button>
      <div className="lightbox-sheet">
        {url ? (
          <iframe className="lightbox-frame" title="Full CV preview" src={`${url}#toolbar=0&view=FitH`} />
        ) : (
          <div className="centered">Rendering your CV…</div>
        )}
      </div>
    </div>,
    window.document.body,
  )
}
