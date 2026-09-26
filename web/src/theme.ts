import { useCallback, useEffect, useState } from 'react'

/**
 * Light or dark chrome for the editor.
 *
 * This is the *app's* appearance only. A CV is a printed page: its own colours
 * live in the document's theme and the rendered PDF is white either way, so
 * nothing here reaches the render path.
 *
 * 'system' is the default and is not a third palette — it simply leaves the
 * root element alone, so the stylesheet's `color-scheme: light dark` follows
 * the OS with no JavaScript involved at all.
 */
export type Appearance = 'system' | 'light' | 'dark'

export const APPEARANCE_KEY = 'rustycv:appearance'

/** Cycle order for the toggle: back to 'system' after the two explicit ones. */
const NEXT: Record<Appearance, Appearance> = {
  system: 'light',
  light: 'dark',
  dark: 'system',
}

export function nextAppearance(current: Appearance): Appearance {
  return NEXT[current]
}

/** Anything unrecognised — or nothing stored — means follow the system. */
export function parseAppearance(stored: string | null): Appearance {
  return stored === 'light' || stored === 'dark' ? stored : 'system'
}

/**
 * The attribute the stylesheet keys on. 'system' removes it rather than
 * writing a value, so the media-query default stays in charge; the same two
 * lines run in the inline script in `index.html`, which applies the stored
 * choice before first paint so the page never flashes the wrong scheme.
 */
export function applyAppearance(appearance: Appearance): void {
  const root = document.documentElement
  if (appearance === 'system') delete root.dataset.appearance
  else root.dataset.appearance = appearance
}

export function useAppearance(): [Appearance, (next: Appearance) => void] {
  // Read straight from storage on mount: the inline script has already applied
  // it to the root element, and this only has to agree with it.
  const [appearance, setAppearance] = useState<Appearance>(() => {
    try {
      return parseAppearance(localStorage.getItem(APPEARANCE_KEY))
    } catch {
      // Private mode, or storage disabled. Following the system is still fine.
      return 'system'
    }
  })

  useEffect(() => {
    applyAppearance(appearance)
  }, [appearance])

  const choose = useCallback((next: Appearance) => {
    setAppearance(next)
    try {
      if (next === 'system') localStorage.removeItem(APPEARANCE_KEY)
      else localStorage.setItem(APPEARANCE_KEY, next)
    } catch {
      // The choice still applies for this session.
    }
  }, [])

  return [appearance, choose]
}
