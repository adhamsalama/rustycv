import { useEffect, useRef, useState } from 'react'
import { useDebounced } from '../hooks/useDebounced'
import type { RasterMessage, RasterRequest, RasterTiming, TextRun } from '../pdfWorker'

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
 * A canvas has no text in it, so selecting, copying and Ctrl+F would all be
 * lost — which is a capability the browser's viewer had. They come back as an
 * invisible layer of positioned spans over each page, the same trick pdf.js's
 * own viewer uses. The geometry is worked out in the worker, because doing it
 * here would mean a second copy of pdf.js in the bundle.
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

// ----------------------------------------------------------- the worker

let rasteriser: Worker | null = null
/** Set once a worker has failed: one bad worker costs the preview once. */
let rasteriserBroken = false
let nextRequest = 1
let nextDocument = 1

/**
 * Which document the worker is holding parsed, as far as this side knows.
 *
 * The worker keeps exactly one, so this is a mirror of it and not a cache in
 * its own right. Being wrong is survivable — the answer comes back `stale`
 * and the bytes are sent again.
 */
let workerHolds: number | null = null

/**
 * A number per `Blob`, so the preview and the expanded view — two components,
 * two widths, one document — are understood to be looking at the same thing.
 */
const documentIds = new WeakMap<Blob, number>()

function documentId(blob: Blob): number {
  let id = documentIds.get(blob)
  if (id === undefined) {
    id = nextDocument++
    documentIds.set(blob, id)
  }
  return id
}

interface Rastered {
  pages: ImageBitmap[]
  text: TextRun[][]
  timing: RasterTiming
}

const waiting = new Map<
  number,
  { resolve: (result: Rastered | 'stale') => void; reject: (error: Error) => void }
>()

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

    if (message.kind === 'rastered')
      pending.resolve({ pages: message.pages, text: message.text, timing: message.timing })
    else if (message.kind === 'stale') pending.resolve('stale')
    else pending.reject(new Error(message.message))
  }

  // A worker that died takes every request with it, including ones that will
  // never now be answered.
  worker.onerror = () => {
    rasteriserBroken = true
    workerHolds = null
    for (const pending of waiting.values()) pending.reject(new Error('The PDF worker stopped'))
    waiting.clear()
  }

  rasteriser = worker
  return worker
}

function ask(request: RasterRequest): Promise<Rastered | 'stale'> {
  const worker = rasteriserWorker()
  return new Promise((resolve, reject) => {
    waiting.set(request.id, { resolve, reject })
    worker.postMessage(request, request.bytes ? [request.bytes] : [])
  })
}

/**
 * Start the worker before there is anything to draw.
 *
 * It is 428kB of pdf.js plus a 1.2MB parser, fetched and compiled the first
 * time a page is rasterised — which without this is the moment the first PDF
 * arrives, so the two queue up instead of overlapping. Called when the editor
 * opens, where it runs alongside fetching the CV and rendering it.
 *
 * Safe to call repeatedly, and safe to call on a browser that cannot use it:
 * the guard is the same one `rasterise` uses.
 */
export function warmRasteriser(): void {
  if (rasteriserBroken || typeof OffscreenCanvas === 'undefined') return
  try {
    rasteriserWorker()
  } catch {
    // Nothing is waiting on this; a worker that will not start is found
    // again, and reported, on the first real render.
  }
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

/**
 * The invisible text over a page.
 *
 * Everything is in percentages of the page box, so the layer needs no
 * measuring of its own and survives the page being laid out at any width —
 * only `font-size` scales with it, through `em` on a container whose own font
 * size is the page height.
 *
 * `scaleX` squeezes each span to the width the PDF says its text is. Without
 * it a line drifts further right with every word, and the selection drifts
 * with it.
 */
function textLayer(runs: TextRun[]): HTMLDivElement {
  const layer = document.createElement('div')
  layer.className = 'pdf-text'
  layer.setAttribute('aria-hidden', 'true')

  for (const run of runs) {
    const span = document.createElement('span')
    span.textContent = run.text
    span.style.left = `${(run.left * 100).toFixed(3)}%`
    span.style.top = `${(run.top * 100).toFixed(3)}%`
    span.style.fontSize = `${run.size.toFixed(5)}em`
    span.style.fontFamily = run.family
    if (run.scaleX !== 1) span.style.setProperty('--scale-x', run.scaleX.toFixed(4))
    if (run.angle !== 0) span.style.setProperty('--angle', `${run.angle.toFixed(2)}deg`)
    if (run.rtl) span.dir = 'rtl'
    layer.append(span)
  }
  return layer
}

/** One page: the raster, and the text you can select on top of it. */
function pageElement(bitmap: ImageBitmap, runs: TextRun[], label: string): HTMLDivElement {
  const wrap = document.createElement('div')
  wrap.className = 'pdf-page'

  const canvas = adopt(bitmap)
  canvas.className = 'pdf-raster'
  canvas.setAttribute('role', 'img')
  canvas.setAttribute('aria-label', label)
  wrap.append(canvas, textLayer(runs))
  return wrap
}

async function rasterise(blob: Blob, width: number, density: number): Promise<Rastered> {
  if (rasteriserBroken || typeof OffscreenCanvas === 'undefined') {
    throw new Error('No off-thread rasteriser')
  }

  const doc = documentId(blob)

  try {
    // Send the bytes only when the worker is not already holding this
    // document. Re-rasterising at a new width is what dragging the window
    // edge does, and re-parsing an unchanged PDF to lay it out wider is
    // precisely the work an iframe's viewer never did.
    const reuse = workerHolds === doc
    let answer = await ask({
      id: nextRequest++,
      doc,
      bytes: reuse ? null : await blob.arrayBuffer(),
      width,
      density,
    })

    if (answer === 'stale') {
      // The worker lost it — restarted, or another document took its place.
      answer = await ask({
        id: nextRequest++,
        doc,
        bytes: await blob.arrayBuffer(),
        width,
        density,
      })
      if (answer === 'stale') throw new Error('The PDF worker will not hold a document')
    }

    workerHolds = doc
    return answer
  } catch (error) {
    // Do not keep paying for a worker that has already proved it cannot do
    // this; the next render goes straight to the iframe.
    rasteriserBroken = true
    throw error
  }
}

/**
 * Off by default, and not a build flag: the question "is the preview slow"
 * gets asked of a running instance, which is usually not a dev server.
 * `localStorage['rustycv:profile'] = '1'`, then reload.
 *
 * The `performance.measure` entries are written either way — they cost
 * nothing and are what the Performance panel reads.
 */
function profiling(): boolean {
  try {
    return localStorage.getItem('rustycv:profile') !== null
  } catch {
    return false
  }
}

// ------------------------------------------------------------- the component

/**
 * How long the width has to hold still before the pages are redrawn.
 *
 * A resize is continuous and every step of it would otherwise re-rasterise
 * every page — the one thing the browser's own viewer did better, because it
 * re-laid out a document it already had.
 */
const RESIZE_SETTLE_MS = 150

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
  const [measured, setMeasured] = useState(0)
  const [failed, setFailed] = useState(false)

  // The settled width, except for the very first one: `useDebounced` starts
  // out holding its initial value, so until it has caught up once the raw
  // measurement stands in and the first preview is not held back 150ms.
  const settled = useDebounced(measured, RESIZE_SETTLE_MS)
  const width = settled || measured

  // `onPages` is called from inside the render effect; holding it in a ref
  // keeps a caller's inline arrow from re-rendering every page.
  const report = useRef(onPages)
  report.current = onPages

  useEffect(() => {
    const element = host.current
    if (!element) return

    const observer = new ResizeObserver((entries) => {
      const next = Math.round(entries[0]?.contentRect.width ?? 0)
      // Still a threshold as well as the debounce: a scrollbar appearing and
      // disappearing is a change the pages need not notice at all.
      setMeasured((current) => (Math.abs(current - next) > 8 ? next : current))
    })
    observer.observe(element)
    return () => observer.disconnect()
  }, [])

  useEffect(() => {
    if (width === 0 || failed) return

    let cancelled = false

    void (async () => {
      const asked = performance.now()
      try {
        // Cap the ratio: a 3x display would quadruple the pixels for a
        // difference nobody can see on a page this size.
        const density = Math.min(window.devicePixelRatio || 1, 2)
        const { pages, text, timing } = await rasterise(blob, width, density)
        if (cancelled) {
          for (const page of pages) page.close()
          return
        }

        const adopted = performance.now()
        const canvases = pages.map((bitmap, index) =>
          pageElement(
            bitmap,
            text[index] ?? [],
            `${label}, page ${index + 1} of ${pages.length}`,
          ),
        )

        report.current?.(canvases.length)

        // Swapped in one call, so the scroll position survives: the old pages
        // are never off the document long enough for the container to collapse
        // and reset. This is what the iframe could not do — changing its src
        // sent every edit back to the top of the CV.
        host.current?.replaceChildren(...canvases)

        const done = performance.now()
        try {
          performance.measure('pdf: adopt', { start: adopted, end: done })
          performance.measure('pdf: preview', { start: asked, end: done })
        } catch {
          // User Timing is not worth failing a preview over.
        }
        if (profiling()) {
          // eslint-disable-next-line no-console
          console.debug(
            `[rustycv] preview ${Math.round(done - asked)}ms` +
              ` = parse ${Math.round(timing.open)} + raster ${Math.round(timing.raster)}` +
              ` + text ${Math.round(timing.text)}` +
              ` + adopt ${Math.round(done - adopted)}` +
              `${timing.reused ? ' (document reused)' : ''}` +
              ` — ${canvases.length} page(s) at ${width}px`,
          )
        }
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
