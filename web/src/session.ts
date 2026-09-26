import { ApiError } from './api'

/** The query `AuthGate` holds. Invalidating it is what re-checks the session. */
export const ME = 'me'

/**
 * Whether a failed query means the session should be re-read.
 *
 * A 401 from anything *else* means the session the gate is holding is stale —
 * it expired while the app was open — so re-reading it drops the app back to
 * the login form instead of leaving a page failing to refresh data nobody is
 * signed in to any more.
 *
 * A 401 from the gate's own query is not that. It is the ordinary answer for a
 * signed-out visitor, and re-reading it on arrival makes the query retrigger
 * itself: refetch, 401, invalidate, refetch. That loop floods the API until the
 * rate limiter stops it, which is how it was found.
 */
export function shouldRecheckSession(error: unknown, queryKey: readonly unknown[]): boolean {
  return error instanceof ApiError && error.status === 401 && queryKey[0] !== ME
}
