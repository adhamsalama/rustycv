import { api, ApiError } from './api'
import type { CvDocument, Diagnostic, RenderMode } from './types'

/**
 * Compiling a CV to a PDF, on whichever machine `/api/config` named.
 *
 * The same Rust renderer either way — `crates/rustycv-wasm` is a shim over the
 * very function the server calls, over the same embedded templates and fonts —
 * so the mode chooses a machine, never an output. The two agree byte for byte,
 * and `a_browser_render_is_the_same_bytes_as_a_server_render` is what keeps
 * them agreeing.
 *
 * Nothing is stored here and there is no way for a user to override the mode.
 * The fallbacks below are the one exception, and they only ever move work
 * *back* to the server — never the other way, which would mean rendering
 * somewhere the operator ruled out.
 */

export interface RenderOutcome {
  blob: Blob
  /** Where it actually happened, which is not always where it was asked to. */
  renderedBy: RenderMode
  /** Set when 'browser' was asked for and the server answered instead. */
  fellBackBecause?: string
}

/**
 * The status a local compile failure carries.
 *
 * 422 is what the server answers for the identical failure, so anything that
 * branches on the status treats a broken template the same whichever machine
 * found out. In particular it is not 401, which the editor reads as "the
 * session is gone" — see `shouldRecheckSession`.
 */
const LOCAL_COMPILE_FAILED = 422

/**
 * Render a document, falling back to the server when the browser cannot.
 *
 * A failure that the server would also hit — a template that does not compile
 * — is *not* a fallback: it is re-thrown as the `ApiError` the remote path
 * would have produced, diagnostics and all, because trying again elsewhere
 * would only spend a round trip to fail the same way.
 */
export async function renderPdf(
  document: CvDocument,
  mode: RenderMode,
  signal?: AbortSignal,
): Promise<RenderOutcome> {
  if (mode === 'server') {
    return { blob: await api.renderPdf(document, signal), renderedBy: 'server' }
  }

  try {
    return { blob: await renderHere(document, signal), renderedBy: 'browser' }
  } catch (error: unknown) {
    if (error instanceof ApiError || isAbort(error)) throw error
    return {
      blob: await api.renderPdf(document, signal),
      renderedBy: 'server',
      fellBackBecause: describe(error),
    }
  }
}

/**
 * Why the browser renderer is out of the running for the rest of this page.
 *
 * Set only for a failure the next attempt would repeat — a module that was
 * never built, or a worker that will not start. A panic is not one of those:
 * it costs the instance, not the feature.
 */
let unavailable: string | null = null

async function renderHere(document: CvDocument, signal?: AbortSignal): Promise<Blob> {
  if (unavailable) throw new Error(unavailable)

  // Imported here rather than at the top of the file so that choosing the
  // server — the default — never pulls the worker into the page at all.
  const local = await import('./localRender')

  let renderer
  try {
    renderer = await local.browserRenderer()
  } catch (error: unknown) {
    unavailable = describe(error)
    throw new Error(unavailable)
  }

  if (!renderer.templates.includes(document.template)) {
    // A module cached from before this template existed. The server has it.
    throw new Error(`the browser renderer does not have the ${document.template} template`)
  }

  try {
    const pdf = await renderer.render(JSON.stringify(document), signal)
    return new Blob([pdf], { type: 'application/pdf' })
  } catch (error: unknown) {
    if (isAbort(error)) throw error

    const failure = classifyLocalFailure(error)
    if (failure.kind === 'document') {
      throw new ApiError(failure.error, LOCAL_COMPILE_FAILED, failure.diagnostics)
    }

    // A panic. Its memory is not to be rendered on top of.
    local.discardBrowserRenderer()
    throw new Error(failure.message)
  }
}

/**
 * Whether a failed local render means "this document does not compile" or
 * "this module is no longer trustworthy".
 *
 * The crate throws its failure body as JSON, so a message that parses into one
 * is the renderer answering; anything else — a wasm trap, a panic message — is
 * the module falling over, and the two want opposite responses.
 */
export type LocalFailure =
  | { kind: 'document'; error: string; diagnostics: Diagnostic[] }
  | { kind: 'module'; message: string }

export function classifyLocalFailure(error: unknown): LocalFailure {
  const message = describe(error)

  let parsed: unknown
  try {
    parsed = JSON.parse(message)
  } catch {
    return { kind: 'module', message }
  }

  if (typeof parsed !== 'object' || parsed === null) return { kind: 'module', message }
  const body = parsed as { error?: unknown; diagnostics?: unknown }
  if (typeof body.error !== 'string') return { kind: 'module', message }

  return {
    kind: 'document',
    error: body.error,
    // Absent for every failure but a compile one — the crate leaves the field
    // out rather than sending an empty list.
    diagnostics: Array.isArray(body.diagnostics) ? (body.diagnostics as Diagnostic[]) : [],
  }
}

function isAbort(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError'
}

function describe(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

// ---------------------------------------------------------------- download

/**
 * The name a downloaded PDF gets.
 *
 * The server writes this into `Content-Disposition`; a browser render has no
 * response to put a header on, so it has to arrive at the same name itself.
 * Mirrors `slug` in `crates/rustycv-server/src/routes.rs` — `the_download_name_matches_the_servers`
 * pins the cases that have ever differed.
 */
export function pdfFilename(fullName: string, title: string): string {
  const source = fullName.trim() === '' ? title : fullName
  const slug = source
    .replace(/[^\p{Alphabetic}\p{N}]/gu, '-')
    .split('-')
    .filter((part) => part !== '')
    .join('-')
    .toLowerCase()

  return `${slug === '' ? 'cv' : slug}.pdf`
}
