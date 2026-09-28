import { useEffect, useRef, useState } from 'react'
import { api } from '../api'
import type { Cv } from '../types'
import { useShare } from './ShareControl'

/**
 * The editor's less-used actions: exporting the JSON and unpublishing. Closes
 * on Escape, a click outside, or picking an item.
 */
export function MoreMenu({ id, cv }: { id: string; cv: Cv | undefined }) {
  const [open, setOpen] = useState(false)
  const wrapper = useRef<HTMLDivElement>(null)

  useEffect(() => {
    if (!open) return
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setOpen(false)
    }
    const onDown = (event: PointerEvent) => {
      if (!wrapper.current?.contains(event.target as Node)) setOpen(false)
    }
    window.addEventListener('keydown', onKey)
    window.addEventListener('pointerdown', onDown)
    return () => {
      window.removeEventListener('keydown', onKey)
      window.removeEventListener('pointerdown', onDown)
    }
  }, [open])

  return (
    <div className="more-menu" ref={wrapper}>
      <button
        type="button"
        className="ghost"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        More
      </button>
      {open ? (
        <div className="more-panel" role="menu">
          <a role="menuitem" className="ghost" href={api.exportUrl(id)} onClick={() => setOpen(false)}>
            Export JSON
          </a>
          {cv?.published ? <Unpublish cv={cv} done={() => setOpen(false)} /> : null}
        </div>
      ) : null}
    </div>
  )
}

function Unpublish({ cv, done }: { cv: Cv; done: () => void }) {
  const { unpublish, pending } = useShare(cv)
  return (
    <button
      type="button"
      role="menuitem"
      className="ghost"
      disabled={pending}
      onClick={() => unpublish.mutate(undefined, { onSettled: done })}
    >
      Unpublish
    </button>
  )
}
