import { useEffect, useRef, useState } from 'react'
import type { RasterMessage, RasterRequest } from '../pdfWorker'

/**
 * The rendered CV, drawn page by page onto canvases.
 *
 * **This is not a second render path.** The bytes are the ones `render_pdf`
 * produced and the ones the download hands over; pdf.js only rasterises them
 * for the screen, the way the browser's viewer was doing before. What changes
 * is who draws the chrome around them.
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
 * Two tiers:
 *
 * 1. **`pdfWorker.ts`** — pdf.js paints onto `OffscreenCanvas` off the main
 *    thread and sends back an `ImageBitmap` per page. Handing one to a
 *    `bitmaprenderer` context is a transfer, not a draw, so the editor's own
 *    thread does no rasterising at all.
 * 2. **The iframe**, toolbar and all. A preview with someone else's furniture
 *    round it beats no preview.
 *
 * There is deliberately no middle tier that paints here instead. It would be
 * a second copy of pdf.js in the bundle — a worker is its own module graph, so
 * nothing is shared — 131kB gzipped for a path nothing reaches: every browser
 * new enough for the `light-dark()` this stylesheet is built on (Safari 17.5)
 * has had `OffscreenCanvas` since 16.4. And painting an A4 page on the thread
 * that answers the keyboard is the thing this file exists to stop.
 */

// -------------------------------------------------------- tier 1: a worker

let rasteriser: Worker | null = null
/** Set once a worker has failed: one bad worker costs the preview once. */
let rasteriserBroken = false
let nextRequest = 1

const waiting = new Map<number, { resolve: (pages: ImageBitmap[]) => void; reject: (error: Error) => void }>()

function rasteriserWorker(): Worker {
  if (rasteriser) return rasteriser

  const worker = new Worker(new URL('../pdfWorker.ts', import.meta.url), { type: 'module' })

  worker.onmessage = (event: MessageEvent<RasterMessage>) => {
    const message = event.data
    const pending = waiting.get(message.id)
    if (!pending) {
      // Nobody is listening any more — an edit landed while this was in
      // flight. Close the bitmaps rather than leaving several megabytes of
      // page to the garbage collector's discretion.
      if (message.kind === 'rastered') for (const page of message.pages) page.close()
      return
    }
    waiting.delete(message.id)
    if (message.kind === 'rastered') pending.resolve(message.pages)
    else pending.reject(new Error(message.message))
  }

  // A worker that died takes every request with it, including ones that will
  // never now be answered.
  worker.onerror = () => {
    rasteriserBroken = true
    for (const pending of waiting.values()) pending.reject(new Error('The PDF worker stopped'))
    waiting.clear()
  }

  rasteriser = worker
  return worker
}

function rasteriseOffThread(
  bytes: ArrayBuffer,
  width: number,
  density: number,
): Promise<ImageBitmap[]> {
  const worker = rasteriserWorker()
  const id = nextRequest++
  return new Promise<ImageBitmap[]>((resolve, reject) => {
    waiting.set(id, { resolve, reject })
    worker.postMessage({ id, bytes, width, density } satisfies RasterRequest, [bytes])
  })
}

/** A canvas that *holds* a bitmap. `transferFromImageBitmap` sizes it too. */
function adopt(bitmap: ImageBitmap): HTMLCanvasElement {
  const canvas = document.createElement('canvas')
  const context = canvas.getContext('bitmaprenderer')
  if (!context) {
    bitmap.close()
    throw new Error('This browser has no bitmaprenderer context')
  }
  context.transferFromImageBitmap(bitmap)
  return canvas
}

// ------------------------------------------------------------- the component

async function rasterise(
  blob: Blob,
  width: number,
  density: number,
): Promise<HTMLCanvasElement[]> {
  if (rasteriserBroken || typeof OffscreenCanvas === 'undefined') {
    throw new Error('No off-thread rasteriser')
  }
  try {
    // A fresh ArrayBuffer: it is transferred to the worker, which detaches it
    // for everybody else — including the second copy of this component that
    // expanding the preview mounts.
    const bitmaps = await rasteriseOffThread(await blob.arrayBuffer(), width, density)
    return bitmaps.map(adopt)
  } catch (error) {
    // Do not keep paying for a worker that has already proved it cannot do
    // this; the next render goes straight to the iframe.
    rasteriserBroken = true
    throw error
  }
}

export function PdfDocument({
  blob,
  url,
  onPages,
  label,
}: {
  blob: Blob
  /** The same bytes as an object URL, for the iframe fallback. */
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
      // Every page is re-rasterised on a width change, so ignore the
      // sub-pixel noise a scrollbar appearing and disappearing produces.
      setWidth((current) => (Math.abs(current - measured) > 8 ? measured : current))
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (width === 0 || failed) return

    let cancelled = false

    void (async () => {
      try {
        // Cap the ratio: a 3x display would quadruple the pixels for a
        // difference nobody can see on a page this size.
        const density = Math.min(window.devicePixelRatio || 1, 2)
        const pages = await rasterise(blob, width, density)
        if (cancelled) return

        report.current?.(pages.length)
        pages.forEach((canvas, index) => {
          canvas.className = 'pdf-page'
          canvas.setAttribute('role', 'img')
          canvas.setAttribute('aria-label', `${label}, page ${index + 1} of ${pages.length}`)
        })

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
    }
  }, [blob, width, failed, label])

  // The browser's viewer, complete with whatever toolbar it insists on.
  if (failed) {
    return <iframe className="preview-frame" title={label} src={`${url}#toolbar=0&view=FitH`} />
  }

  return (
    <div className="pdf-scroll">
      <div className="pdf-pages" ref={host} />
    </div>
  )
}
