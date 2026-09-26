import { useState } from 'react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import { AppearanceToggle } from '../components/AppearanceToggle'
import { ME } from '../session'

/**
 * One form for both signing in and signing up.
 *
 * Two pages would mean two routes to keep in step for a form with two fields,
 * and someone who mistypes their address on the sign-in form is one click from
 * creating the account they meant to sign in to.
 */
export function Login() {
  const queryClient = useQueryClient()
  const [isNew, setIsNew] = useState(false)
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')

  const submit = useMutation({
    mutationFn: () => (isNew ? api.signup(email, password) : api.login(email, password)),
    // The session cookie is set by the response, so re-reading `me` is all it
    // takes for the gate to let the app through.
    onSuccess: (user) => queryClient.setQueryData([ME], user),
  })

  return (
    <main className="auth-page">
      <div className="auth-corner">
        <AppearanceToggle />
      </div>

      <form
        className="auth-card"
        onSubmit={(e) => {
          e.preventDefault()
          submit.mutate()
        }}
      >
        <h1>RustyCV</h1>
        <p className="muted">
          {isNew
            ? 'Create an account to keep your CVs and applications.'
            : 'Sign in to your CVs and applications.'}
        </p>

        <label className="field">
          <span className="field-label">Email</span>
          <input
            type="email"
            autoComplete="username"
            required
            value={email}
            onChange={(e) => setEmail(e.target.value)}
          />
        </label>

        <label className="field">
          <span className="field-label">Password</span>
          <input
            type="password"
            /* Tells a password manager whether to offer a saved password or a
               generated one, which is the one thing the two modes differ on. */
            autoComplete={isNew ? 'new-password' : 'current-password'}
            required
            minLength={8}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>

        {submit.isError ? (
          <p className="auth-error" role="alert">
            {(submit.error as Error).message}
          </p>
        ) : null}

        <button type="submit" className="primary" disabled={submit.isPending}>
          {submit.isPending ? 'One moment…' : isNew ? 'Create account' : 'Sign in'}
        </button>

        <button
          type="button"
          className="ghost auth-switch"
          onClick={() => {
            setIsNew(!isNew)
            submit.reset()
          }}
        >
          {isNew ? 'I already have an account' : 'Create an account instead'}
        </button>
      </form>
    </main>
  )
}
