import { nextAppearance, useAppearance, type Appearance } from '../theme'

const FACE: Record<Appearance, { icon: string; label: string }> = {
  system: { icon: '◐', label: 'System' },
  light: { icon: '☀', label: 'Light' },
  dark: { icon: '☾', label: 'Dark' },
}

/**
 * One button that cycles System → Light → Dark.
 *
 * A cycle rather than a three-way control because it lives in a crowded
 * toolbar and is pressed once in a while, not adjusted — the icon says where
 * it is and the label says where the next press goes.
 */
export function AppearanceToggle() {
  const [appearance, choose] = useAppearance()
  const next = nextAppearance(appearance)

  return (
    <button
      type="button"
      className="ghost appearance-toggle"
      onClick={() => choose(next)}
      title={`Appearance: ${FACE[appearance].label} — switch to ${FACE[next].label.toLowerCase()}`}
      aria-label={`Appearance: ${FACE[appearance].label}. Switch to ${FACE[next].label.toLowerCase()}.`}
    >
      <span aria-hidden="true">{FACE[appearance].icon}</span>
    </button>
  )
}
