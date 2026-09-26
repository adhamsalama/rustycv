import { describe, expect, it } from 'vitest'
import { classifyLocalFailure, parseRenderMode, pdfFilename } from './renderer'

describe('the renderer preference', () => {
  it('renders on the server for anything it does not recognise', () => {
    expect(parseRenderMode(null)).toBe('server')
    expect(parseRenderMode('')).toBe('server')
    // A value left by an older build, or edited by hand.
    expect(parseRenderMode('gpu')).toBe('server')
    expect(parseRenderMode('browser')).toBe('browser')
  })
})

/**
 * The distinction the fallback turns on. A document that will not compile
 * fails the same way on the server, so it is shown; a module that fell over
 * is a reason to go and ask the server, which is a different thing entirely.
 */
describe('classifying a failed local render', () => {
  it('reads the crate’s failure body as the document’s problem', () => {
    const failure = classifyLocalFailure(
      new Error(
        JSON.stringify({
          error: 'the template failed to compile',
          diagnostics: [{ severity: 'error', message: 'unknown variable', line: 12, hints: [] }],
        }),
      ),
    )

    expect(failure.kind).toBe('document')
    if (failure.kind !== 'document') return
    expect(failure.error).toBe('the template failed to compile')
    expect(failure.diagnostics).toHaveLength(1)
  })

  it('reads a failure with no diagnostics as the document’s problem too', () => {
    // `unknown template` is one of these: the crate leaves the field out
    // rather than sending an empty list.
    const failure = classifyLocalFailure(new Error('{"error":"unknown template: nope"}'))
    expect(failure).toEqual({
      kind: 'document',
      error: 'unknown template: nope',
      diagnostics: [],
    })
  })

  it('reads a trap or a panic as the module’s problem', () => {
    expect(classifyLocalFailure(new Error('unreachable executed')).kind).toBe('module')
    expect(classifyLocalFailure(new Error('memory access out of bounds')).kind).toBe('module')
    expect(classifyLocalFailure('the browser renderer is not loaded').kind).toBe('module')
  })

  it('does not mistake other JSON for a failure body', () => {
    // A panic message that happens to parse is still a panic. Only an object
    // carrying a string `error` is the renderer answering.
    expect(classifyLocalFailure(new Error('42')).kind).toBe('module')
    expect(classifyLocalFailure(new Error('null')).kind).toBe('module')
    expect(classifyLocalFailure(new Error('{"panicked":true}')).kind).toBe('module')
  })
})

/**
 * The server writes the download's name into `Content-Disposition`; a render
 * that happens here has to arrive at the same one. These are the cases from
 * `the_download_name_matches_the_editors` in `crates/rustycv-server`, and the
 * two lists are meant to be read together.
 */
describe('the downloaded filename', () => {
  it('matches the name the server would have sent', () => {
    expect(pdfFilename('Adham Salama', 'Backend CV')).toBe('adham-salama.pdf')
    expect(pdfFilename('R&D Lead', 'x')).toBe('r-d-lead.pdf')
    expect(pdfFilename('  Ada   Lovelace  ', 'x')).toBe('ada-lovelace.pdf')
    // Unicode letters and digits survive; everything else is a separator.
    expect(pdfFilename('José Ñuñez 3', 'x')).toBe('josé-ñuñez-3.pdf')
  })

  it('falls back to the CV’s name, and then to "cv"', () => {
    expect(pdfFilename('', 'My Résumé')).toBe('my-résumé.pdf')
    expect(pdfFilename('   ', 'My Résumé')).toBe('my-résumé.pdf')
    // A name with nothing alphanumeric in it leaves no slug to use.
    expect(pdfFilename('!!!', 'ignored')).toBe('cv.pdf')
    expect(pdfFilename('', '')).toBe('cv.pdf')
  })
})
