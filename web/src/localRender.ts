import type { RenderedPdf, RenderRequest, WorkerMessage } from './renderWorker'

/**
 * The browser render path: one worker, one wasm module, reused.
 *
 * Starting the module costs a multi-megabyte download and a moment of
 * instantiation, so it happens once per page and the instance is kept. The
 * exception is a panic, which leaves wasm memory in a state the next render
 * would inherit — [`discardBrowserRenderer`] is how the caller throws that
 * instance away rather than rendering on top of it.
 */
export interface BrowserRenderer {
  /**
   * The template ids this module was built with.
   *
   * A browser can hold a cached module from before a deployment added a
   * template, and the caller checks this rather than letting the render come
   * back as a compile error the user can do nothing about.
   */
  templates: string[]
  render(documentJson: string, signal?: AbortSignal): Promise<RenderedPdf>
  dispose(): void
}

let instance: Promise<BrowserRenderer> | null = null

/** The module, started on the first call and shared by every one after it. */
export function browserRenderer(): Promise<BrowserRenderer> {
  instance ??= spawn()
  return instance
}

/** Drop the current instance. The next call starts a fresh one. */
export function discardBrowserRenderer(): void {
  const dying = instance
  instance = null
  dying?.then((renderer) => renderer.dispose()).catch(() => {
    // It never started; there is nothing to tear down.
  })
}

function spawn(): Promise<BrowserRenderer> {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./renderWorker.ts', import.meta.url), { type: 'module' })
    const waiting = new Map<
      number,
      { resolve: (pdf: RenderedPdf) => void; reject: (e: Error) => void }
    >()
    let nextId = 1

    const fail = (message: string) => {
      worker.terminate()
      for (const pending of waiting.values()) pending.reject(new Error(message))
      waiting.clear()
      reject(new Error(message))
    }

    worker.onmessage = (event: MessageEvent<WorkerMessage>) => {
      const message = event.data
      if (message.kind === 'ready') {
        resolve({
          templates: message.templates,
          dispose: () => worker.terminate(),
          render: (documentJson, signal) =>
            new Promise<RenderedPdf>((resolveRender, rejectRender) => {
              if (signal?.aborted) return rejectRender(abortError(signal))

              const id = nextId++
              waiting.set(id, { resolve: resolveRender, reject: rejectRender })

              // A running render cannot be interrupted — the worker is inside
              // synchronous wasm. Abandoning it is enough: dropping the id is
              // what makes its eventual answer land nowhere.
              signal?.addEventListener(
                'abort',
                () => {
                  if (waiting.delete(id)) rejectRender(abortError(signal))
                },
                { once: true },
              )

              worker.postMessage({ id, documentJson } satisfies RenderRequest)
            }),
        })
        return
      }

      if (message.kind === 'unavailable') return fail(message.message)

      const pending = waiting.get(message.id)
      if (!pending) return // Abandoned while it was running.
      waiting.delete(message.id)
      if (message.kind === 'rendered') pending.resolve(message.pdf)
      else pending.reject(new Error(message.message))
    }

    // A worker that will not even start — no module support, a blocked URL —
    // reads the same to the caller as one whose wasm was never built.
    worker.onerror = (event) => fail(event.message || 'the render worker failed to start')
  })
}

function abortError(signal: AbortSignal): Error {
  // `signal.reason` is a DOMException named AbortError unless the caller
  // supplied its own, and callers check that name to tell a superseded render
  // from a failed one.
  return signal.reason instanceof Error
    ? signal.reason
    : new DOMException('The render was aborted', 'AbortError')
}
