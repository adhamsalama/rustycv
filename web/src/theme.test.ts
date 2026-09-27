import { describe, expect, it } from 'vitest'
import CSS from './styles.css?raw'
import {
  DEFAULT_PALETTE,
  PALETTES,
  deriveAccent,
  parseAccent,
  parseAppearance,
  parsePalette,
} from './theme'

/**
 * `import CSS from './styles.css?raw'` returns an **empty string** unless
 * vitest is told to process CSS — and an empty string is not an error
 * anywhere downstream: `''.indexOf(x)` is -1, `slice(-1)` is one character,
 * and one character contains no colours. That is how the audit at the bottom
 * of this file passed for its whole life without reading a single rule.
 * `css: true` in vite.config.ts is the fix; this is the guard that notices if
 * it is ever taken away again.
 */
describe('this file', () => {
  it('has a stylesheet to audit at all', () => {
    expect(CSS.length).toBeGreaterThan(5000)
    expect(CSS).toContain('==== palette tokens: end')
  })
})

describe('the appearance preference', () => {
  it('falls back to the system for anything it does not recognise', () => {
    expect(parseAppearance(null)).toBe('system')
    expect(parseAppearance('')).toBe('system')
    // A value left by an older build, or edited by hand.
    expect(parseAppearance('solarized')).toBe('system')
    expect(parseAppearance('dark')).toBe('dark')
  })
})

describe('the palette preference', () => {
  it('falls back to the default for anything it does not recognise', () => {
    expect(parsePalette(null)).toBe(DEFAULT_PALETTE)
    // A palette that was removed, or one that never existed.
    expect(parsePalette('dracula')).toBe(DEFAULT_PALETTE)
    expect(parsePalette('gruvbox')).toBe('gruvbox')
  })
})

/**
 * One picked colour has to serve a cream page and a near-black one. The two
 * cases that matter are the ones a colour input makes easy to reach: a colour
 * too dark to see on a dark scheme, and one too light to see on a light one.
 */
describe('a chosen accent', () => {
  /** The two halves of a derived accent, or a failure naming the input. */
  function halves(hex: string): [light: string, dark: string] {
    const accent = deriveAccent(hex)?.accent ?? ''
    const match = /light-dark\((#[0-9a-f]{6}), (#[0-9a-f]{6})\)/.exec(accent)
    const light = match?.[1]
    const dark = match?.[2]
    if (light === undefined || dark === undefined) throw new Error(`not a pair: ${hex} -> ${accent}`)
    return [light, dark]
  }

  const value = (hex: string) => Number.parseInt(hex.slice(1), 16)

  it('lightens a colour that would vanish on a dark page', () => {
    const [light, dark] = halves('#101020')
    // Already dark enough for a pale page, lifted well clear for a deep one.
    expect(light).toBe('#101020')
    expect(value(dark)).toBeGreaterThan(value('#101020'))
  })

  it('darkens a colour that would vanish on a light page', () => {
    const [light, dark] = halves('#fff8b0')
    expect(value(light)).toBeLessThan(value('#fff8b0'))
    expect(dark).toBe('#fff8b0')
  })

  it('picks ink that can be read on it, per scheme', () => {
    // Always this pair, and that is the point: the two luminance bounds sit
    // either side of the ink threshold, so whatever gets picked, the light
    // half is dark enough for white ink and the dark half light enough for
    // black. A loosened bound shows up here first.
    expect(deriveAccent('#101020')?.ink).toBe('light-dark(#ffffff, #12141a)')
    expect(deriveAccent('#fff8b0')?.ink).toBe('light-dark(#ffffff, #12141a)')
    expect(deriveAccent('#8da101')?.ink).toBe('light-dark(#ffffff, #12141a)')
  })

  it('always hands the dark scheme the lighter of the two', () => {
    // No colour comes back unchanged from both halves, because no single
    // luminance reads on both a cream page and a near-black one — which is
    // the whole reason a picked accent is worked into a pair.
    for (const hex of ['#101020', '#fff8b0', '#8da101', '#777777', '#e0435a']) {
      const [light, dark] = halves(hex)
      expect({ hex, lighter: value(dark) > value(light) }).toEqual({ hex, lighter: true })
    }
  })

  it('refuses anything that is not a colour', () => {
    expect(deriveAccent('rebeccapurple')).toBeNull()
    expect(deriveAccent('#12345')).toBeNull()
    expect(deriveAccent('')).toBeNull()
    // Three-digit hex is what a hand-edited entry might hold.
    expect(deriveAccent('#0a0')?.hex).toBe('#00aa00')
  })

  it('survives a stored value that is missing, damaged or the wrong shape', () => {
    expect(parseAccent(null)).toBeNull()
    expect(parseAccent('not json')).toBeNull()
    expect(parseAccent('"a string"')).toBeNull()
    expect(parseAccent('{"hex":42}')).toBeNull()
    expect(parseAccent('{"hex":"#8da101"}')?.hex).toBe('#8da101')
  })
})

/**
 * A palette is a flat list of the same token names, and nothing else. One that
 * leaves a token out does not fail — it silently inherits the default's, so a
 * Gruvbox editor would be Gruvbox with one Everforest border in it, which is
 * exactly the kind of thing nobody notices until they are looking at it.
 */
describe('the palettes', () => {
  const region = CSS.slice(
    CSS.indexOf('==== palette tokens: start'),
    CSS.indexOf('==== palette tokens: end'),
  )

  const blocks = new Map<string, string[]>()
  for (const block of region.matchAll(/\[data-palette='(\w+)'\][^{]*\{([^}]*)\}/g)) {
    const [, id, body] = block
    if (id === undefined || body === undefined) continue
    const tokens = [...body.matchAll(/(--[\w-]+):/g)]
      .flatMap((token) => (token[1] === undefined ? [] : [token[1]]))
      .sort()
    blocks.set(id, tokens)
  }

  it('are the same set the picker offers', () => {
    expect([...blocks.keys()].sort()).toEqual(PALETTES.map((palette) => palette.id).sort())
  })

  it('each define every base token', () => {
    const base = blocks.get(DEFAULT_PALETTE) ?? []
    expect(base.length).toBeGreaterThan(8)
    for (const [id, tokens] of blocks) {
      expect({ id, tokens }).toEqual({ id, tokens: base })
    }
  })

  it('define nothing a palette has no business setting', () => {
    // Shadows, fonts, radii and everything derived live in the shared block.
    // A palette that set one would be a palette with a rule of its own.
    for (const [id, tokens] of blocks) {
      const stray = tokens.filter((token) =>
        /shadow|font|radius|ease|text-|accent-soft|accent-wash|hover|notice|stripe|warn/.test(token),
      )
      expect({ id, stray }).toEqual({ id, stray: [] })
    }
  })
})

/**
 * The audit that keeps dark mode from rotting: a colour written as a literal
 * outside the token block is a colour that only works in one scheme. Adding a
 * rule with `background: #fff` is exactly how half a UI stays light, and it is
 * invisible in review until someone switches over.
 *
 * Everything it allows is listed below with why.
 */
const SCHEME_AGNOSTIC = [
  // Paper. The CV is a printed page and is white in both schemes.
  '#fff',
  // Ink on a scrim dark enough for it in either scheme (badges, the expand
  // control), and the scrims and overlay shadows themselves.
  'rgba(0, 0, 0, 0.62)',
  'rgba(0, 0, 0, 0.8)',
  'rgba(0, 0, 0, 0.25)',
  'rgba(0, 0, 0, 0.3)',
  'rgba(0, 0, 0, 0.35)',
  'rgba(24, 26, 30, 0.55)',
  'rgba(24, 26, 30, 0.4)',
]

describe('the stylesheet', () => {
  it('names no scheme-specific colour outside the tokens', () => {
    // Everything after the sentinel is ordinary rules. The tokens above it are
    // `light-dark()` pairs and `color-mix()` of them by construction.
    const body = CSS.slice(CSS.indexOf('==== palette tokens: end'))

    const literals = [...body.matchAll(/#[0-9a-fA-F]{3,8}\b|rgba?\([^)]*\)/g)].map(
      (match) => match[0],
    )

    const offenders = literals.filter((literal) => !SCHEME_AGNOSTIC.includes(literal))
    expect(offenders).toEqual([])
  })
})
