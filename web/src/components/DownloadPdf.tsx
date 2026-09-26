import { useState } from 'react'
import { api } from '../api'
import { pdfFilename, renderPdf } from '../renderer'
import type { CvDocument, RenderMode } from '../types'

/**
 * Save the CV as a PDF.
 *
 * On the server path this stays an ordinary link: middle-click, "save link
 * as" and a copied URL all keep working, and the filename rides along in the
 * response's `Content-Disposition`. A render that happens here has no response
 * to hang a header on, so that path has to build both the file and its name —
 * which is what `pdfFilename` is for.
 *
 * The two also differ in *which* document they save. The link downloads what
 * the server holds; rendering here saves what is on screen, including an edit
 * the autosave has not settled yet. That is the same document the preview
 * beside it is showing, which is the more useful of the two to agree with.
 */
export function DownloadPdf({
  id,
  document: cv,
  title,
  mode,
}: {
  id: string
  document: CvDocument
  title: string
  mode: RenderMode
}) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  if (mode === 'server') {
    return (
      <a className="primary" href={api.pdfUrl(id)}>
        Download PDF
      </a>
    )
  }

  const save = async () => {
    setBusy(true)
    setError(null)
    try {
      const { blob } = await renderPdf(cv, mode)
      const url = URL.createObjectURL(blob)
      const link = window.document.createElement('a')
      link.href = url
      link.download = pdfFilename(cv.basics.fullName, title)
      link.click()
      // Revoking while the browser is still reading the blob cancels the
      // download, and "still reading" is not something it tells us about.
      window.setTimeout(() => URL.revokeObjectURL(url), 30_000)
    } catch (failure: unknown) {
      setError(failure instanceof Error ? failure.message : 'Could not render the PDF')
    } finally {
      setBusy(false)
    }
  }

  return (
    <button
      type="button"
      className="primary"
      disabled={busy}
      onClick={() => void save()}
      title={error ?? undefined}
    >
      {busy ? 'Rendering…' : error ? 'Download failed' : 'Download PDF'}
    </button>
  )
}
