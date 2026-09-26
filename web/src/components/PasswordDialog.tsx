import { useEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import { useMutation } from '@tanstack/react-query'
import { api } from '../api'

/**
 * Change your own password.
 *
 * A dialog rather than a settings page: it is one form with three fields and
 * nothing else belongs beside it yet, and a page would mean a route that the
 * signed-out landing page would then have to know not to claim.
 */
export function PasswordDialog({ onClose }: { onClose: () => void }) {
  const firstField = useRef<HTMLInputElement>(null)
  const [current, setCurrent] = useState('')
  const [next, setNext] = useState('')
  const [confirmation, setConfirmation] = useState('')
  const [done, setDone] = useState(false)

  // The two copies must agree before anything is sent: the server has no way
  // to catch a typo that was made identically once.
  const mismatched = confirmation !== '' && next !== confirmation

  const change = useMutation({
    mutationFn: () => api.changePassword(current, next),
    onSuccess: () => setDone(true),
  })

  useEffect(() => {
    firstField.current?.focus()

    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return createPortal(
    <div
      className="scrim"
      role="dialog"
      aria-modal="true"
      aria-label="Change password"
      onMouseDown={(e) => {
        // Only a click that both starts and ends on the backdrop closes, so a
        // drag out of a field does not discard what was typed.
        if (e.target === e.currentTarget) onClose()
      }}
    >
      <form
        className="dialog-card"
        onSubmit={(e) => {
          e.preventDefault()
          if (!mismatched) change.mutate()
        }}
      >
        <h2>Change password</h2>

        {done ? (
          <>
            <p className="muted">
              Done. Any other browser or device you were signed in on has been signed out.
            </p>
            <button type="button" className="primary" onClick={onClose}>
              Close
            </button>
          </>
        ) : (
          <>
            <label className="field">
              <span className="field-label">Current password</span>
              <input
                ref={firstField}
                type="password"
                autoComplete="current-password"
                required
                value={current}
                onChange={(e) => setCurrent(e.target.value)}
              />
            </label>

            <label className="field">
              <span className="field-label">New password</span>
              <input
                type="password"
                autoComplete="new-password"
                required
                minLength={8}
                value={next}
                onChange={(e) => setNext(e.target.value)}
              />
            </label>

            <label className="field">
              <span className="field-label">New password again</span>
              <input
                type="password"
                autoComplete="new-password"
                required
                value={confirmation}
                onChange={(e) => setConfirmation(e.target.value)}
              />
            </label>

            {mismatched ? (
              <p className="auth-error" role="alert">
                The two new passwords do not match.
              </p>
            ) : null}

            {change.isError ? (
              <p className="auth-error" role="alert">
                {(change.error as Error).message}
              </p>
            ) : null}

            <p className="muted small">
              Signing in elsewhere will stop working — this session stays.
            </p>

            <div className="dialog-actions">
              <button type="button" className="ghost" onClick={onClose}>
                Cancel
              </button>
              <button
                type="submit"
                className="primary"
                disabled={change.isPending || mismatched}
              >
                {change.isPending ? 'One moment…' : 'Change password'}
              </button>
            </div>
          </>
        )}
      </form>
    </div>,
    window.document.body,
  )
}
