/// <reference lib="webworker" />

/**
 * The Typst compiler, running off the main thread.
 *
 * Laying out a CV is a few hundred milliseconds of straight computation. On
 * the main thread that is a few hundred milliseconds the editor does not
 * answer the keyboard, on every edit — which would make rendering here cost
 * more than the round trip it saves.
 *
 * The worker is single-threaded, so requests queue behind whichever render is
 * running with no queueing code of its own.
 */

/**
 * Bytes the glue has already copied out of wasm memory into an `ArrayBuffer`
 * of their own — which is what makes them transferable, and what lets a `Blob`
 * be built straight from them.
 */
export type RenderedPdf = Uint8Array<ArrayBuffer>

/** One document to render. `id` is what pairs a request with its answer. */
export interface RenderRequest {
  id: number
  documentJson: string
}

export type WorkerMessage =
  /** Sent once, unprompted, when the module is up — or when it is not coming. */
  | { kind: 'ready'; templates: string[] }
  | { kind: 'unavailable'; message: string }
  | { kind: 'rendered'; id: number; pdf: RenderedPdf }
  | { kind: 'failed'; id: number; message: string }

/**
 * Where `wasm-bindgen --target web` writes its glue.
 *
 * Served as a plain asset out of `public/` and imported at runtime rather than
 * bundled, so the editor builds and runs whether or not anyone has built the
 * wasm. A module that was never built is a failed import here, and the server
 * goes on doing the rendering.
 */
const GLUE_URL = '/wasm/rustycv_wasm.js'

/** The three exports of `crates/rustycv-wasm`, as the glue presents them. */
interface Glue {
  default: () => Promise<unknown>
  renderPdf: (documentJson: string) => RenderedPdf
  templateIds: () => string
}

let glue: Glue | null = null

async function start(): Promise<void> {
  const module = (await import(/* @vite-ignore */ GLUE_URL)) as Glue
  await module.default()
  glue = module
  self.postMessage({
    kind: 'ready',
    templates: JSON.parse(module.templateIds()) as string[],
  } satisfies WorkerMessage)
}

start().catch((error: unknown) => {
  self.postMessage({ kind: 'unavailable', message: describe(error) } satisfies WorkerMessage)
})

self.onmessage = (event: MessageEvent<RenderRequest>) => {
  const { id, documentJson } = event.data

  if (!glue) {
    self.postMessage({
      kind: 'failed',
      id,
      message: 'the browser renderer is not loaded',
    } satisfies WorkerMessage)
    return
  }

  try {
    const pdf = glue.renderPdf(documentJson)
    // Transfer rather than copy: the PDF is the biggest thing that crosses
    // here and this side has no further use for it.
    self.postMessage({ kind: 'rendered', id, pdf } satisfies WorkerMessage, [pdf.buffer])
  } catch (error: unknown) {
    // Either the document does not compile — in which case this message is the
    // JSON failure body the crate threw — or the module panicked and this is
    // whatever the runtime said. The main thread is where the two are told
    // apart, because it is the side that can act on the difference.
    self.postMessage({ kind: 'failed', id, message: describe(error) } satisfies WorkerMessage)
  }
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}
