import { useEffect, useRef, useState } from 'react'
import { api, ApiError } from '../api'
import type { CvDocument, Diagnostic } from '../types'
import { useDebounced } from '../hooks/useDebounced'

/** How long the editor must be idle before a render is worth spending. */
const DEBOUNCE_MS = 400

interface PreviewState {
  url: string | null
  pending: boolean
  diagnostics: Diagnostic[]
  error: string | null
}

export function PdfPreview({ document, revision }: { document: CvDocument; revision: number }) {
  // Debounce the revision counter rather than the document: it is a number, so
  // the effect below doesn't re-run on every structurally-equal re-render.
  const settledRevision = useDebounced(revision, DEBOUNCE_MS)
  const [state, setState] = useState<PreviewState>({
    url: null,
    pending: true,
    diagnostics: [],
    error: null,
  })

  // Read the document without making it an effect dependency — the revision is
  // what says the content changed.
  const documentRef = useRef(document)
  documentRef.current = document

  useEffect(() => {
    const controller = new AbortController()
    let cancelled = false
    setState((s) => ({ ...s, pending: true }))

    api
      .renderPdf(documentRef.current, controller.signal)
      .then((blob) => {
        if (cancelled) return
        const url = URL.createObjectURL(blob)
        setState((previous) => {
          // Swap first, then revoke: releasing the old URL before the iframe has
          // the new one makes the preview flash blank on every edit.
          if (previous.url) URL.revokeObjectURL(previous.url)
          return { url, pending: false, diagnostics: [], error: null }
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
  }, [settledRevision])

  // Release the final object URL when the preview unmounts.
  const urlRef = useRef<string | null>(null)
  urlRef.current = state.url
  useEffect(() => () => void (urlRef.current && URL.revokeObjectURL(urlRef.current)), [])

  return (
    <div className="preview">
      <div className="preview-status">
        {state.pending ? <span className="badge">Rendering…</span> : null}
        {state.error ? <span className="badge badge-error">{state.error}</span> : null}
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
        <iframe className="preview-frame" title="CV preview" src={`${state.url}#toolbar=0&view=FitH`} />
      ) : (
        <div className="preview-empty">{state.error ? 'No preview' : 'Rendering your CV…'}</div>
      )}
    </div>
  )
}
