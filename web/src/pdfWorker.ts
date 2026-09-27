/// <reference lib="webworker" />

import * as pdfjs from 'pdfjs-dist'
import pdfjsWorkerUrl from 'pdfjs-dist/build/pdf.worker.min.mjs?url'

/**
 * Turning the rendered PDF into page images, off the main thread.
 *
 * pdf.js splits its work in two: the *parser* runs in a worker of its own,
 * but painting the operator list onto a canvas happens wherever the document
 * proxy lives. Hold that proxy on the main thread and every re-render
 * rasterises an A4 page there — while an iframe's viewer, in Chrome at least,
 * was doing it in another process and costing the editor nothing.
 *
 * So the proxy lives here instead, painting onto `OffscreenCanvas`, and what
 * goes back is an `ImageBitmap` per page. Handing one to a canvas with a
 * `bitmaprenderer` context is a transfer, not a draw: the main thread does no
 * rasterising at all.
 */

/** One rendered CV to turn into page images. `id` pairs it with its answer. */
export interface RasterRequest {
  id: number
  /** The PDF, transferred — this buffer is detached once it is posted. */
  bytes: ArrayBuffer
  /** CSS pixels the page is laid out at. */
  width: number
  /** Device pixels per CSS pixel, already capped by the caller. */
  density: number
}

export type RasterMessage =
  | { kind: 'rastered'; id: number; pages: ImageBitmap[] }
  | { kind: 'failed'; id: number; message: string }

interface CanvasAndContext {
  canvas: OffscreenCanvas | null
  context: OffscreenCanvasRenderingContext2D | null
}

/**
 * pdf.js makes canvases of its own for soft masks, tiling patterns and
 * transparency groups. Its default factory reaches for `document`, which does
 * not exist in here, so it gets one that makes `OffscreenCanvas` instead.
 *
 * Duck-typed rather than extended: `BaseCanvasFactory` is not part of
 * pdfjs-dist's public exports, and the three methods below are its whole
 * surface.
 */
class OffscreenCanvasFactory {
  create(width: number, height: number): CanvasAndContext {
    if (width <= 0 || height <= 0) throw new Error('Invalid canvas size')
    const canvas = new OffscreenCanvas(width, height)
    return { canvas, context: canvas.getContext('2d', { willReadFrequently: true }) }
  }

  reset(target: CanvasAndContext, width: number, height: number): void {
    if (!target.canvas) throw new Error('Canvas is not specified')
    if (width <= 0 || height <= 0) throw new Error('Invalid canvas size')
    target.canvas.width = width
    target.canvas.height = height
  }

  destroy(target: CanvasAndContext): void {
    if (!target.canvas) throw new Error('Canvas is not specified')
    target.canvas.width = 0
    target.canvas.height = 0
    target.canvas = null
    target.context = null
  }
}

/**
 * pdf.js's parsing worker, nested inside this one.
 *
 * Constructed here rather than left to pdf.js, which reads `window.location`
 * to decide whether its worker script is same-origin — and there is no
 * `window` in a worker. Handing it a ready-made port skips that path.
 *
 * One for the life of this worker: the document changes on every edit, the
 * thing parsing it does not, and a `PDFWorker` per render would mean spawning
 * and terminating one over a 1.2MB script on every pause in typing.
 */
let parser: pdfjs.PDFWorker | null = null

function parsingWorker(): pdfjs.PDFWorker {
  // Cast through `unknown`: pdf.js's generated types declare `port` as
  // `null | undefined`, which is a JSDoc artefact rather than the contract —
  // `PDFWorker.create` looks the port up in a WeakMap and uses it.
  parser ??= new pdfjs.PDFWorker({
    port: new Worker(pdfjsWorkerUrl, { type: 'module' }),
  } as unknown as ConstructorParameters<typeof pdfjs.PDFWorker>[0])
  return parser
}

const describe = (error: unknown): string =>
  error instanceof Error ? error.message : String(error)

self.onmessage = async (event: MessageEvent<RasterRequest>) => {
  const { id, bytes, width, density } = event.data

  const task = pdfjs.getDocument({
    data: bytes,
    worker: parsingWorker(),
    CanvasFactory: OffscreenCanvasFactory,
    // No DOM in here, so no `@font-face` to install a font into: pdf.js draws
    // glyphs from the embedded font programs instead. Typst embeds every font
    // it uses, so there is nothing this could fail to find.
    disableFontFace: true,
    isOffscreenCanvasSupported: true,
  })

  try {
    const document_ = await task.promise
    const pages: ImageBitmap[] = []

    for (let number = 1; number <= document_.numPages; number += 1) {
      const page = await document_.getPage(number)
      const unscaled = page.getViewport({ scale: 1 })
      const viewport = page.getViewport({ scale: (width / unscaled.width) * density })
      const canvas = new OffscreenCanvas(
        Math.floor(viewport.width),
        Math.floor(viewport.height),
      )

      await page.render({
        canvas: canvas as unknown as HTMLCanvasElement,
        viewport,
      }).promise

      pages.push(canvas.transferToImageBitmap())
    }

    // Transferred, not copied: an A4 page at 2x is about 5MB and there may be
    // several of them.
    self.postMessage({ kind: 'rastered', id, pages } satisfies RasterMessage, pages)
  } catch (error) {
    self.postMessage({ kind: 'failed', id, message: describe(error) } satisfies RasterMessage)
  } finally {
    // Only the document: `task._worker` is null because the parser above was
    // passed in, so this leaves it running for the next edit.
    await task.destroy()
  }
}
