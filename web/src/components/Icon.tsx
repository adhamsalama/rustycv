/**
 * The interface's own icons.
 *
 * Separate from `assets/icons`, which are Font Awesome glyphs compiled into
 * the renderer and drawn *inside* a CV. These are chrome, they are never
 * rendered to a PDF, and they exist because `☰ ⠿ ⤢ ✕` are not icons — they
 * are whatever glyph the platform happens to have for that code point, at
 * whatever weight, and they read as fallback characters rather than controls.
 *
 * One stroked 24-unit grid so every icon shares a weight. Drawn with
 * `currentColor`, so a button's own colour and its transitions carry them.
 */
const PATHS = {
  menu: 'M4 6h16M4 12h16M4 18h16',
  back: 'M19 12H5M12 19l-7-7 7-7',
  eye: 'M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0z',
  'eye-off':
    'M10.6 6.2A10 10 0 0 1 12 6c6.5 0 10 6 10 6a18 18 0 0 1-3 3.6M6.6 6.6A18 18 0 0 0 2 12s3.5 7 10 7a10 10 0 0 0 4.2-.9M3 3l18 18',
  trash:
    'M3 6h18M8 6V4.5A1.5 1.5 0 0 1 9.5 3h5A1.5 1.5 0 0 1 16 4.5V6M6.5 6l1 13.2A1.8 1.8 0 0 0 9.3 21h5.4a1.8 1.8 0 0 0 1.8-1.8L17.5 6',
  plus: 'M12 5v14M5 12h14',
  close: 'M18 6 6 18M6 6l12 12',
  expand: 'M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7',
  sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10z M12 1v2M12 21v2M4.2 4.2l1.4 1.4M18.4 18.4l1.4 1.4M1 12h2M21 12h2M4.2 19.8l1.4-1.4M18.4 5.6l1.4-1.4',
  moon: 'M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z',
  monitor: 'M3 4h18v12H3zM8 20h8M12 16v4',
  palette:
    'M12 3a9 9 0 0 0 0 18 2 2 0 0 0 1.6-3.2 2 2 0 0 1 1.6-3.2H18a3 3 0 0 0 3-3 9 9 0 0 0-9-8.6z M7.5 10.5h.01M10.5 7h.01M15 7.5h.01',
} as const

export type IconName = keyof typeof PATHS

/**
 * `aria-hidden` always: every caller is a button that carries its own label,
 * and an icon that announced itself as well would say everything twice.
 */
export function Icon({ name, className = 'icon' }: { name: IconName; className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.75"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      <path d={PATHS[name]} />
    </svg>
  )
}

/**
 * The drag handle, which is dots rather than strokes and so does not fit the
 * grid above.
 */
export function GripIcon() {
  return (
    <svg className="icon icon-sm" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" focusable="false">
      <circle cx="9" cy="6" r="1.5" />
      <circle cx="9" cy="12" r="1.5" />
      <circle cx="9" cy="18" r="1.5" />
      <circle cx="15" cy="6" r="1.5" />
      <circle cx="15" cy="12" r="1.5" />
      <circle cx="15" cy="18" r="1.5" />
    </svg>
  )
}
