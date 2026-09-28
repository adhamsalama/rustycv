/**
 * The mark and the name: a page with its rusted corner peeling off.
 *
 * Inline rather than `<img src="/logo.svg">` so it can take the page's colours
 * — the outline is `currentColor` and the paper is the surface, which is what
 * lets it survive dark mode. The rust is `--rust`, one colour whatever the
 * palette, because it is the brand and not a theme. `public/logo.svg` is the
 * same drawing with the colours written in, for places outside the app.
 */
export function Logo() {
  return (
    <span className="logo">
      <svg viewBox="0 0 64 64" aria-hidden="true">
        <path
          d="M14 4h26l14 14v38a4 4 0 0 1-4 4H14a4 4 0 0 1-4-4V8a4 4 0 0 1 4-4z"
          fill="var(--raised)"
          stroke="currentColor"
          strokeWidth="4"
          strokeLinejoin="round"
        />
        <path
          d="M40 4v10a4 4 0 0 0 4 4h10z"
          fill="var(--rust)"
          stroke="currentColor"
          strokeWidth="4"
          strokeLinejoin="round"
        />
        <circle cx="35" cy="10" r="1.8" fill="var(--rust)" />
        <circle cx="46" cy="25" r="1.5" fill="var(--rust)" />
        <rect x="18" y="26" width="18" height="5" rx="2.5" fill="var(--rust)" />
        <rect x="18" y="36" width="28" height="4" rx="2" fill="currentColor" />
        <rect x="18" y="44" width="24" height="4" rx="2" fill="currentColor" opacity=".5" />
        <rect x="18" y="51" width="26" height="4" rx="2" fill="currentColor" opacity=".5" />
      </svg>
      <span>
        <span className="logo-rust">Rusty</span>CV
      </span>
    </span>
  )
}
