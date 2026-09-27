import { useEffect, useRef, useState } from 'react'

/**
 * The rendered CV, drawn page by page onto canvases.
 *
 * **This is not a second render path.** The bytes are the ones `render_pdf`
 * produced and the ones the download hands over; pdf.js only rasterises them
 * for the screen, the way the browser's own viewer was doing before. What
 * changes is who draws the chrome around them.
 *
 * An `<iframe>` gets the *browser's* PDF viewer, and `#toolbar=0` is a
 * Chromium parameter — Firefox's pdf.js ignores it, so a Firefox user was
 * given a search box, a page spinner, a zoom menu and five annotation tools
 * for a document that is thrown away on the next keystroke. Safari ignores it
 * too. Drawing the pages ourselves is the only way the preview looks the same
 * everywhere, and it hands us three things the iframe could not: the page
 * count, a scroll position that survives a re-render, and a page that can have
 * a shadow under it.
 *
 * If any of that fails the iframe comes back, toolbar and all — a preview with
 * someone else's furniture around it beats no preview.
 */

/** Resolved once per page load: pdf.js and its worker are megabytes, and only
 *  the editor's preview ever wants them. */
let pdfjs: Promise<typeof import('pdfjs-dist')> | null = null

function loadPdfjs(): Promise<typeof import('pdfjs-dist')> {
  pdfjs ??= (async () => {
    const [lib, worker] = await Promise.all([
      import('pdfjs-dist'),
      import('pdfjs-dist/build/pdf.worker.min.mjs?url'),
    ])
    lib.GlobalWorkerOptions.workerSrc = worker.default
    return lib
  })()
  return pdfjs
}

export function PdfDocument({
  blob,
  url,
  onPages,
  label,
}: {
  blob: Blob
  /** The same bytes as an object URL, for the fallback below. */
  url: string
  onPages?: (pages: number) => void
  label: string
}) {
  const host = useRef<HTMLDivElement>(null)
  const [width, setWidth] = useState(0)
  const [failed, setFailed] = useState(false)

  // `onPages` is called from inside the render effect; holding it in a ref
  // keeps a caller's inline arrow from re-rendering every page.
  const report = useRef(onPages)
  report.current = onPages

  useEffect(() => {
    const element = host.current
    if (!element) return

    const observer = new ResizeObserver((entries) => {
      const measured = Math.round(entries[0]?.contentRect.width ?? 0)
      // A canvas is re-rasterised on every width change, so ignore the
      // sub-pixel noise a scrollbar appearing and disappearing produces.
      setWidth((current) => (Math.abs(current - measured) > 8 ? measured : current))
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (width === 0 || failed) return

    let cancelled = false
    // The loading task, not the document: `destroy` lives on the task in
    // pdf.js 6 and is what shuts the worker down.
    let task: import('pdfjs-dist').PDFDocumentLoadingTask | null = null

    void (async () => {
      try {
        const lib = await loadPdfjs()
        // A fresh ArrayBuffer per call: pdf.js may transfer the one it is
        // given to its worker, which detaches it for everybody else —
        // including the second copy of this component that expanding mounts.
        task = lib.getDocument({ data: await blob.arrayBuffer() })
        const document_ = await task.promise
        if (cancelled) return
        report.current?.(document_.numPages)

        // Cap the ratio: a 3x display would quadruple the pixels for a
        // difference nobody can see on a page this size.
        const density = Math.min(window.devicePixelRatio || 1, 2)
        const pages: HTMLCanvasElement[] = []

        for (let number = 1; number <= document_.numPages; number += 1) {
          const page = await document_.getPage(number)
          if (cancelled) return
          const unscaled = page.getViewport({ scale: 1 })
          const viewport = page.getViewport({ scale: (width / unscaled.width) * density })

          const canvas = window.document.createElement('canvas')
          canvas.className = 'pdf-page'
          canvas.width = Math.floor(viewport.width)
          canvas.height = Math.floor(viewport.height)
          canvas.setAttribute('role', 'img')
          canvas.setAttribute(
            'aria-label',
            `${label}, page ${number} of ${document_.numPages}`,
          )

          await page.render({ canvas, viewport }).promise
          if (cancelled) return
          pages.push(canvas)
        }

        // Swapped in one call, so the scroll position survives: the old pages
        // are never off the document long enough for the container to collapse
        // and reset. This is what the iframe could not do — changing its src
        // sent every edit back to the top of the CV.
        host.current?.replaceChildren(...pages)
      } catch (error) {
        if (cancelled) return
        // A cancelled render task throws on the way out; that is not a failure.
        if (error instanceof Error && error.name === 'RenderingCancelledException') return
        setFailed(true)
      }
    })()

    return () => {
      cancelled = true
      void task?.destroy()
    }
  }, [blob, width, failed, label])

  // The browser's viewer, complete with whatever toolbar it insists on. Worth
  // having: it is the difference between an ugly preview and none.
  if (failed) {
    return <iframe className="preview-frame" title={label} src={`${url}#toolbar=0&view=FitH`} />
  }

  return (
    <div className="pdf-scroll">
      <div className="pdf-pages" ref={host} />
    </div>
  )
}
