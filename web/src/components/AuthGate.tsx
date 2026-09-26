import type { ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { ApiError, api } from '../api'
import { Login } from '../pages/Login'
import { ME } from '../session'

/**
 * Nothing inside renders until there is a session.
 *
 * The gate is one query rather than a flag in a store, so every page shares the
 * same answer and a session that expires mid-edit drops the whole app back to
 * the form instead of leaving half of it showing stale data.
 */
export function AuthGate({ children }: { children: ReactNode }) {
  const { data: user, isLoading } = useQuery({
    queryKey: [ME],
    queryFn: api.me,
    // A 401 here is the answer, not a failure: retrying it just delays the
    // login form. Anything else is worth one retry.
    retry: (count, error) => !(error instanceof ApiError && error.status === 401) && count < 1,
  })

  // No spinner: the round trip is local and a flash of "Loading…" before a
  // form reads worse than a beat of nothing.
  if (isLoading) return null

  return user ? <>{children}</> : <Login />
}
