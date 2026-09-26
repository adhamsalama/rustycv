import type { RenderMode } from '../renderer'

const FACE: Record<RenderMode, { icon: string; label: string }> = {
  server: { icon: '☁', label: 'On the server' },
  browser: { icon: '⚡', label: 'In this browser' },
}

/**
 * One button that flips where PDFs are compiled.
 *
 * Both settings produce the same PDF — the browser runs the same Rust renderer
 * compiled to wasm — so the title says what the choice actually costs and buys
 * rather than describing two outputs, which would be a lie.
 */
export function RendererToggle({
  mode,
  onChange,
}: {
  mode: RenderMode
  onChange: (next: RenderMode) => void
}) {
  const next: RenderMode = mode === 'server' ? 'browser' : 'server'
  const explain =
    next === 'browser'
      ? 'Switch to rendering here — a one-off download, then no round trip per edit'
      : 'Switch back to rendering on the server'

  return (
    <button
      type="button"
      className="ghost renderer-toggle"
      onClick={() => onChange(next)}
      title={`Rendering ${FACE[mode].label.toLowerCase()}. ${explain}.`}
      aria-label={`Rendering ${FACE[mode].label.toLowerCase()}. ${explain}.`}
    >
      <span aria-hidden="true">{FACE[mode].icon}</span>
    </button>
  )
}
