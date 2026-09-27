import { useCallback, useEffect, useState } from 'react'

/**
 * How the editor looks. Three independent choices, all stored locally:
 *
 * - **appearance** — light, dark, or follow the OS.
 * - **palette** — which set of colours, of which light/dark are two halves.
 * - **accent** — one colour of the user's own, overriding the palette's.
 *
 * All three are the *app's* appearance only. A CV is a printed page: its own
 * colours live in the document's theme and the rendered PDF is white either
 * way, so nothing here reaches the render path.
 *
 * Each is stored under its own key and applied by its own function, because
 * the inline script in `index.html` has to reapply all three before the first
 * paint and reads them one at a time. Nothing in here is sent to the server —
 * a preference that followed the account would be a migration and a route for
 * something that is only ever true of the machine you are sitting at.
 */

export type Appearance = 'system' | 'light' | 'dark'

export const APPEARANCE_KEY = 'rustycv:appearance'
export const PALETTE_KEY = 'rustycv:palette'
export const ACCENT_KEY = 'rustycv:accent'

/** Anything unrecognised — or nothing stored — means follow the system. */
export function parseAppearance(stored: string | null): Appearance {
  return stored === 'light' || stored === 'dark' ? stored : 'system'
}

/**
 * The attribute the stylesheet keys on. 'system' removes it rather than
 * writing a value, so the media-query default stays in charge.
 */
export function applyAppearance(appearance: Appearance): void {
  const root = document.documentElement
  if (appearance === 'system') delete root.dataset.appearance
  else root.dataset.appearance = appearance
}

// ------------------------------------------------------------- palettes

export type PaletteId =
  | 'everforest'
  | 'gruvbox'
  | 'rosepine'
  | 'kanagawa'
  | 'violet'
  | 'classic'

/**
 * The default, and the one `:root` carries. Like 'system' above, choosing it
 * *removes* the attribute rather than writing it, so the stylesheet needs no
 * rule that repeats the default's values.
 */
export const DEFAULT_PALETTE: PaletteId = 'everforest'

export const PALETTES: { id: PaletteId; label: string }[] = [
  { id: 'everforest', label: 'Everforest' },
  { id: 'gruvbox', label: 'Gruvbox' },
  { id: 'rosepine', label: 'Rosé Pine' },
  { id: 'kanagawa', label: 'Kanagawa' },
  { id: 'violet', label: 'Violet' },
  { id: 'classic', label: 'Classic' },
]

const PALETTE_IDS = new Set<string>(PALETTES.map((palette) => palette.id))

export function parsePalette(stored: string | null): PaletteId {
  return stored !== null && PALETTE_IDS.has(stored) ? (stored as PaletteId) : DEFAULT_PALETTE
}

export function applyPalette(palette: PaletteId): void {
  const root = document.documentElement
  if (palette === DEFAULT_PALETTE) delete root.dataset.palette
  else root.dataset.palette = palette
}

// --------------------------------------------------------------- accent

/**
 * A chosen accent, already worked out for both schemes.
 *
 * `accent` and `ink` are finished `light-dark()` values rather than the raw
 * hex, because they are what gets written to the root element — and because
 * the inline script in `index.html` applies them before the first paint and
 * must not have to carry a copy of the arithmetic below. `hex` is kept so the
 * colour input can be reopened on what was actually picked.
 */
export interface AccentChoice {
  hex: string
  accent: string
  ink: string
}

type Rgb = [number, number, number]

function parseHex(value: string): Rgb | null {
  const digits = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(value)?.[1]
  if (digits === undefined) return null
  const full = digits.length === 3 ? digits.replace(/./g, (digit) => digit + digit) : digits
  const packed = Number.parseInt(full, 16)
  return [(packed >> 16) & 0xff, (packed >> 8) & 0xff, packed & 0xff]
}

const toHex = (rgb: Rgb): string =>
  '#' + rgb.map((channel) => Math.round(channel).toString(16).padStart(2, '0')).join('')

/** WCAG relative luminance. Used only to compare, never reported. */
function luminance([r, g, b]: Rgb): number {
  const channel = (value: number) => {
    const v = value / 255
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  }
  return 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b)
}

const mix = ([r, g, b]: Rgb, [tr, tg, tb]: Rgb, amount: number): Rgb => [
  r + (tr - r) * amount,
  g + (tg - g) * amount,
  b + (tb - b) * amount,
]

const BLACK: Rgb = [0, 0, 0]
const WHITE: Rgb = [255, 255, 255]

/**
 * Walk a colour towards black or white until it is dark or light enough to be
 * read against the scheme it will sit on.
 *
 * In steps rather than in one jump because the target is a luminance and the
 * distance to it depends on the hue: 10% of the way to white moves a yellow
 * much further than it moves a navy. Bounded so a colour that cannot reach the
 * target — there isn't one, but the loop should not depend on that — stops.
 */
function toward(rgb: Rgb, target: Rgb, until: (value: number) => boolean): Rgb {
  let current = rgb
  for (let step = 0; step < 24 && !until(luminance(current)); step += 1) {
    current = mix(current, target, 0.1)
  }
  return current
}

/** Black or white, whichever can be read *on* this colour. */
const inkFor = (rgb: Rgb): string => (luminance(rgb) > 0.4 ? '#12141a' : '#ffffff')

/**
 * Work one picked colour into an accent for both schemes.
 *
 * A single hex cannot serve both: #1a1a2e is a fine accent on paper and
 * invisible on a dark page, and #ffe08a is the same problem the other way
 * round. So the light scheme gets it darkened until it reads on a pale
 * surface and the dark scheme gets it lightened until it reads on a deep one —
 * a colour already in range comes back from both unchanged.
 *
 * Returns `null` for anything that is not a hex colour, which is what the
 * colour input emits and what an edited `localStorage` entry might not be.
 */
export function deriveAccent(hex: string): AccentChoice | null {
  const rgb = parseHex(hex)
  if (!rgb) return null

  // The bounds are read off the built-in palettes rather than invented: their
  // light accents sit around 0.15–0.33 and their dark ones around 0.45–0.58,
  // so a picked colour that lands outside those is the one that will look
  // muddier than anything the app ships. They also straddle `inkFor`'s
  // threshold from both sides, which is what makes the ink pair below always
  // come out white-on-light and dark-on-dark.
  const light = toward(rgb, BLACK, (value) => value <= 0.36)
  const dark = toward(rgb, WHITE, (value) => value >= 0.45)

  return {
    hex: toHex(rgb),
    accent: `light-dark(${toHex(light)}, ${toHex(dark)})`,
    ink: `light-dark(${inkFor(light)}, ${inkFor(dark)})`,
  }
}

/** What was stored, or `null` for nothing stored and for anything broken. */
export function parseAccent(stored: string | null): AccentChoice | null {
  if (!stored) return null
  try {
    const parsed: unknown = JSON.parse(stored)
    if (typeof parsed !== 'object' || parsed === null) return null
    const { hex } = parsed as { hex?: unknown }
    // Re-derive rather than trusting the stored pair: the arithmetic may have
    // changed since it was written, and the hex is the only part a person
    // actually chose.
    return typeof hex === 'string' ? deriveAccent(hex) : null
  } catch {
    return null
  }
}

/**
 * Write the override, or take it away.
 *
 * Removing the properties rather than restoring the palette's values is what
 * makes "Match palette" work without knowing which palette is on: the
 * stylesheet's `var(--accent-user, var(--accent-base))` falls back on its own.
 */
export function applyAccent(choice: AccentChoice | null): void {
  const style = document.documentElement.style
  if (!choice) {
    style.removeProperty('--accent-user')
    style.removeProperty('--accent-ink-user')
    return
  }
  style.setProperty('--accent-user', choice.accent)
  style.setProperty('--accent-ink-user', choice.ink)
}

// ---------------------------------------------------------------- hooks

/** Read once on mount: the inline script has already applied it. */
function stored<T>(key: string, parse: (value: string | null) => T): T {
  try {
    return parse(localStorage.getItem(key))
  } catch {
    // Private mode, or storage disabled. The default is still fine.
    return parse(null)
  }
}

function remember(key: string, value: string | null): void {
  try {
    if (value === null) localStorage.removeItem(key)
    else localStorage.setItem(key, value)
  } catch {
    // The choice still applies for this session.
  }
}

export function useAppearance(): [Appearance, (next: Appearance) => void] {
  const [appearance, setAppearance] = useState<Appearance>(() =>
    stored(APPEARANCE_KEY, parseAppearance),
  )

  useEffect(() => {
    applyAppearance(appearance)
  }, [appearance])

  const choose = useCallback((next: Appearance) => {
    setAppearance(next)
    remember(APPEARANCE_KEY, next === 'system' ? null : next)
  }, [])

  return [appearance, choose]
}

export function usePalette(): [PaletteId, (next: PaletteId) => void] {
  const [palette, setPalette] = useState<PaletteId>(() => stored(PALETTE_KEY, parsePalette))

  useEffect(() => {
    applyPalette(palette)
  }, [palette])

  const choose = useCallback((next: PaletteId) => {
    setPalette(next)
    remember(PALETTE_KEY, next === DEFAULT_PALETTE ? null : next)
  }, [])

  return [palette, choose]
}

export function useAccent(): [AccentChoice | null, (hex: string | null) => void] {
  const [accent, setAccent] = useState<AccentChoice | null>(() => stored(ACCENT_KEY, parseAccent))

  useEffect(() => {
    applyAccent(accent)
  }, [accent])

  const choose = useCallback((hex: string | null) => {
    const next = hex === null ? null : deriveAccent(hex)
    setAccent(next)
    remember(ACCENT_KEY, next === null ? null : JSON.stringify(next))
  }, [])

  return [accent, choose]
}
