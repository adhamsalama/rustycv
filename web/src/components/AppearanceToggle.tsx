import { useEffect, useRef, useState } from 'react'
import {
  DEFAULT_PALETTE,
  PALETTES,
  useAccent,
  useAppearance,
  usePalette,
  type Appearance,
} from '../theme'
import { Icon, type IconName } from './Icon'

interface Scheme {
  id: Appearance
  label: string
  icon: IconName
}

const SYSTEM: Scheme = { id: 'system', label: 'System', icon: 'monitor' }
const SCHEMES: Scheme[] = [
  SYSTEM,
  { id: 'light', label: 'Light', icon: 'sun' },
  { id: 'dark', label: 'Dark', icon: 'moon' },
]

/**
 * Everything about how the editor looks, behind one button.
 *
 * A popover rather than a dialog: all three controls change the page behind
 * them, and a scrim over that page would hide the only feedback any of them
 * has. It closes on Escape, on a click outside, and on nothing else — picking
 * a palette and then an accent is one visit, not two.
 *
 * Kept under the old component's name and with no props, so the four headers
 * that already place it need no changes.
 */
export function AppearanceToggle() {
  const [appearance, chooseAppearance] = useAppearance()
  const [palette, choosePalette] = usePalette()
  const [accent, chooseAccent] = useAccent()
  const [open, setOpen] = useState(false)
  const wrapper = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return

    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    // `mousedown`, not `click`: dragging the colour input's slider and
    // releasing outside the panel would otherwise close it mid-pick.
    const onDown = (event: MouseEvent) => {
      if (!wrapper.current?.contains(event.target as Node)) setOpen(false)
    }

    window.addEventListener('keydown', onKey)
    window.addEventListener('mousedown', onDown)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('mousedown', onDown)
    }
  }, [open])

  const current = SCHEMES.find((scheme) => scheme.id === appearance) ?? SYSTEM

  return (
    <div className="appearance" ref={wrapper}>
      <button
        type="button"
        className="ghost appearance-toggle"
        aria-expanded={open}
        aria-label="Appearance"
        title="Appearance"
        onClick={() => setOpen((was) => !was)}
      >
        <Icon name={current.icon} />
      </button>

      {open ? (
        <div className="appearance-panel" role="group" aria-label="Appearance">
          <div className="appearance-group">
            <span>Scheme</span>
            <div className="appearance-scheme">
              {SCHEMES.map((scheme) => (
                <button
                  key={scheme.id}
                  type="button"
                  aria-pressed={appearance === scheme.id}
                  title={scheme.label}
                  onClick={() => chooseAppearance(scheme.id)}
                >
                  <Icon name={scheme.icon} className="icon icon-sm" />
                  {scheme.label}
                </button>
              ))}
            </div>
          </div>

          <div className="appearance-group">
            <span>Palette</span>
            <div className="palette-grid">
              {PALETTES.map((option) => (
                <button
                  key={option.id}
                  type="button"
                  className="palette-swatch"
                  /* Wearing the palette is what paints the swatch: the
                     stylesheet's `[data-palette]` blocks are not anchored to
                     :root, so this button's subtree resolves that palette's
                     own tokens rather than a hardcoded copy of them. */
                  data-palette={option.id}
                  aria-pressed={palette === option.id}
                  onClick={() => choosePalette(option.id)}
                >
                  <span className="palette-bars" aria-hidden="true">
                    <i style={{ background: 'var(--bg)' }} />
                    <i style={{ background: 'var(--surface)' }} />
                    <i style={{ background: 'var(--accent-base)' }} />
                  </span>
                  {option.label}
                </button>
              ))}
            </div>
          </div>

          <div className="appearance-group">
            <span>Accent</span>
            <div className="accent-row">
              <input
                type="color"
                /* The palette's own accent is a `light-dark()` pair and this
                   input takes one colour, so with nothing overridden it shows
                   a neutral rather than pretending to report what is on. */
                value={accent?.hex ?? '#808080'}
                aria-label="Accent colour"
                onChange={(event) => chooseAccent(event.target.value)}
              />
              <button
                type="button"
                className="ghost"
                disabled={accent === null}
                title={
                  accent === null
                    ? 'The accent is the palette’s own'
                    : 'Go back to the palette’s own accent'
                }
                onClick={() => chooseAccent(null)}
              >
                {accent === null ? 'Palette accent' : 'Match palette'}
              </button>
            </div>
          </div>

          {palette !== DEFAULT_PALETTE || accent !== null ? (
            <button
              type="button"
              className="ghost small"
              onClick={() => {
                choosePalette(DEFAULT_PALETTE)
                chooseAccent(null)
              }}
            >
              Reset to defaults
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
