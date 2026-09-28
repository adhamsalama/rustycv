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

/**
 * One rasterising job. `id` pairs it with its answer.
 *
 * `bytes` is sent **once per document**. Re-rasterising the same PDF at a new
 * width — which is what dragging the window edge does — reuses the parsed
 * document already here and sends `null`, because re-parsing a document that
 * has not changed to lay it out slightly wider is exactly the work an
 * iframe's viewer never did.
 */
export interface RasterRequest {
  id: number
  /** Which document these pages come from. */
  doc: number
  bytes: ArrayBuffer | null
  /** CSS pixels the page is laid out at. */
  width: number
  /** Device pixels per CSS pixel, already capped by the caller. */
  density: number
}

/**
 * One run of text, positioned in *fractions of the page*.
 *
 * Fractions rather than pixels so the same geometry survives a resize: the
 * page is re-rasterised at the new width but these need no recomputing.
 *
 * `scaleX` is the correction that makes a span the width the PDF says the
 * text is, rather than the width the browser's fallback font happens to
 * render it at. Without it selection drifts further right along every line —
 * it is what pdf.js's own text layer spends its `measureText` on.
 */
export interface TextRun {
  text: string
  /** Fraction of the page's width and height, from the top left. */
  left: number
  top: number
  /** Font height as a fraction of the page *width*, which is the dimension
   *  the layer's container query measures. */
  size: number
  family: string
  scaleX: number
  /** Degrees, for the rare rotated run. */
  angle: number
  rtl: boolean
}

/** How long each half took, for the profiling the editor can switch on. */
export interface RasterTiming {
  /** Parsing. Zero when the document was already open. */
  open: number
  /** Painting every page. */
  raster: number
  /** Pulling the text out of them, for the selectable layer. */
  text: number
  reused: boolean
}

export type RasterMessage =
  | {
      kind: 'rastered'
      id: number
      pages: ImageBitmap[]
      /** One list per page, same order. */
      text: TextRun[][]
      timing: RasterTiming
    }
  /** The document asked for is not the one held. Send the bytes again. */
  | { kind: 'stale'; id: number }
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

/**
 * The document currently parsed, and which one it is.
 *
 * Exactly one: the editor previews a single CV, and the expanded view is the
 * same document at a different width. Holding more would mean deciding when to
 * let go of PDFs nobody is looking at.
 */
let open: { doc: number; task: pdfjs.PDFDocumentLoadingTask; proxy: pdfjs.PDFDocumentProxy } | null =
  null

async function openDocument(doc: number, bytes: ArrayBuffer) {
  // Already here. The expanded preview sends the bytes for a document the
  // inline one has just opened, because neither knows about the other; the
  // copy it sent is simply dropped.
  if (open?.doc === doc) return open

  // Destroy the old one first: its pages hold on to decoded images, and two
  // CVs' worth is twice what anybody is looking at.
  const previous = open
  open = null
  await previous?.task.destroy()

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
  open = { doc, task, proxy: await task.promise }
  return open
}

/**
 * Where every run of text sits on the page, as the DOM will need it.
 *
 * Worked out here rather than on the main thread because the arithmetic wants
 * pdf.js — `Util.transform`, the text content, the font metrics — and pdf.js
 * is already loaded in this worker. Importing it on the other side to do the
 * same sums would put a second 437kB copy in the bundle (131kB gzipped, and
 * it does not tree-shake: measured). What crosses the wire instead is a list
 * of numbers the main thread can turn into spans without knowing anything
 * about PDFs.
 *
 * The maths mirrors pdf.js's own text layer: transform the item into page
 * space, take the font height off the transform, lift the baseline by the
 * font's ascent, and measure the string to find how far off the rendered
 * width is.
 */
interface RawDims {
  pageWidth: number
  pageHeight: number
  pageX: number
  pageY: number
}

async function textRuns(
  page: pdfjs.PDFPageProxy,
  viewport: pdfjs.PageViewport,
): Promise<TextRun[]> {
  const content = await page.getTextContent()
  // `rawDims` is typed as a bare `Object` by pdf.js's generated types; it is
  // four numbers, and `TextLayer` reads exactly these.
  const { pageWidth, pageHeight, pageX, pageY } = viewport.rawDims as RawDims
  // The page's own transform at scale 1: flip the y axis and move the origin
  // to the top left, which is where the DOM measures from.
  const flip = [1, 0, 0, -1, -pageX, pageY + pageHeight]

  // Any size will do — `scaleX` is a ratio of widths, so it does not depend
  // on the one measured at.
  const ruler = new OffscreenCanvas(1, 1).getContext('2d')
  const MEASURE_AT = 100

  const runs: TextRun[] = []
  for (const item of content.items) {
    if (!('str' in item) || item.str === '') continue

    const style = content.styles[item.fontName] as
      | { fontFamily?: string; ascent?: number; vertical?: boolean }
      | undefined
    const tx = pdfjs.Util.transform(flip, item.transform)

    let angle = Math.atan2(tx[1], tx[0])
    if (style?.vertical) angle += Math.PI / 2

    const height = Math.hypot(tx[2], tx[3])
    // pdf.js measures a font's ascent when the metric is missing; 0.8 is the
    // default it falls back to, and every font Typst embeds reports one.
    const ascent = height * (style?.ascent ?? 0.8)
    const family = style?.fontFamily ?? 'sans-serif'

    const left = angle === 0 ? tx[4] : tx[4] + ascent * Math.sin(angle)
    const top = angle === 0 ? tx[5] - ascent : tx[5] - ascent * Math.cos(angle)

    let scaleX = 1
    const target = style?.vertical ? item.height : item.width
    if (ruler && target > 0 && item.str.length > 0) {
      ruler.font = `${MEASURE_AT}px ${family}`
      const drawn = ruler.measureText(item.str).width
      // `target` is in page units and `drawn` at MEASURE_AT px, so put them
      // on the same footing before dividing.
      if (drawn > 0) scaleX = (target * (MEASURE_AT / height)) / drawn
    }

    runs.push({
      text: item.str,
      left: left / pageWidth,
      top: top / pageHeight,
      size: height / pageWidth,
      family,
      scaleX,
      angle: angle === 0 ? 0 : angle * (180 / Math.PI),
      rtl: item.dir === 'rtl',
    })
  }
  return runs
}

const describe = (error: unknown): string =>
  error instanceof Error ? error.message : String(error)

/** Named so they are findable in the Performance panel's worker track. */
function measure(name: string, start: number, end: number): number {
  try {
    performance.measure(name, { start, end })
  } catch {
    // User Timing is not worth failing a render over.
  }
  return end - start
}

/**
 * One job at a time.
 *
 * `renderWorker.ts` gets away without this because its handler is
 * synchronous, but an `async` one yields at every `await` and the next message
 * starts running inside it. Two overlapping jobs share one parsed document —
 * so the second would destroy and reopen the document the first was still
 * rasterising pages from. The preview and the expanded view are exactly that
 * pair: two components, two widths, one PDF.
 */
let queue: Promise<void> = Promise.resolve()

self.onmessage = (event: MessageEvent<RasterRequest>) => {
  queue = queue.then(() => handle(event.data))
}

async function handle({ id, doc, bytes, width, density }: RasterRequest): Promise<void> {
  try {
    if (bytes === null && open?.doc !== doc) {
      // The editor believed this document was still here — a worker restart,
      // or another one took its place. It will send the bytes again.
      self.postMessage({ kind: 'stale', id } satisfies RasterMessage)
      return
    }

    const started = performance.now()
    const held = bytes === null ? open : await openDocument(doc, bytes)
    if (!held) throw new Error('No document to rasterise')
    const opened = performance.now()

    const pages: ImageBitmap[] = []
    const text: TextRun[][] = []
    for (let number = 1; number <= held.proxy.numPages; number += 1) {
      const page = await held.proxy.getPage(number)
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
    const rastered = performance.now()

    for (let number = 1; number <= held.proxy.numPages; number += 1) {
      const page = await held.proxy.getPage(number)
      text.push(await textRuns(page, page.getViewport({ scale: 1 })))
    }
    const extracted = performance.now()

    const timing: RasterTiming = {
      open: bytes === null ? 0 : measure('pdf: parse', started, opened),
      raster: measure('pdf: rasterise', opened, rastered),
      text: measure('pdf: text', rastered, extracted),
      reused: bytes === null,
    }

    // Transferred, not copied: an A4 page at 2x is about 5MB and there may be
    // several of them.
    self.postMessage({ kind: 'rastered', id, pages, text, timing } satisfies RasterMessage, pages)
  } catch (error) {
    // A document that failed half-way through opening is not one to keep.
    open = null
    self.postMessage({ kind: 'failed', id, message: describe(error) } satisfies RasterMessage)
  }
}
