import { describe, expect, it } from 'vitest'
import CSS from './styles.css?raw'
import { nextAppearance, parseAppearance } from './theme'

describe('the appearance preference', () => {
  it('cycles back to following the system', () => {
    expect(nextAppearance('system')).toBe('light')
    expect(nextAppearance('light')).toBe('dark')
    expect(nextAppearance('dark')).toBe('system')
  })

  it('falls back to the system for anything it does not recognise', () => {
    expect(parseAppearance(null)).toBe('system')
    expect(parseAppearance('')).toBe('system')
    // A value left by an older build, or edited by hand.
    expect(parseAppearance('solarized')).toBe('system')
    expect(parseAppearance('dark')).toBe('dark')
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
  // control), and the scrims themselves.
  'rgba(0, 0, 0, 0.62)',
  'rgba(0, 0, 0, 0.8)',
  'rgba(0, 0, 0, 0.25)',
  'rgba(0, 0, 0, 0.35)',
  'rgba(24, 26, 30, 0.55)',
]

describe('the stylesheet', () => {
  it('names no scheme-specific colour outside the tokens', () => {
    // Everything before the first `}` after the token block is the :root
    // declaration, which is all `light-dark()` pairs by construction.
    const body = CSS.slice(CSS.indexOf('--radius'))

    const literals = [
      ...body.matchAll(/#[0-9a-fA-F]{3,8}\b|rgba?\([^)]*\)/g),
    ].map((match) => match[0])

    const offenders = literals.filter((literal) => !SCHEME_AGNOSTIC.includes(literal))
    expect(offenders).toEqual([])
  })
})
