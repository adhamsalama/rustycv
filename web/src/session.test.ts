import { describe, expect, it } from 'vitest'
import { ApiError } from './api'
import { ME, shouldRecheckSession } from './session'

describe('deciding to re-read the session', () => {
  const unauthorized = new ApiError('sign in to continue', 401)

  it('re-reads it when another query finds the session gone', () => {
    expect(shouldRecheckSession(unauthorized, ['cvs'])).toBe(true)
    expect(shouldRecheckSession(unauthorized, ['applications'])).toBe(true)
  })

  it('does not re-read it because the session query itself said 401', () => {
    // The regression this file exists for. A signed-out visitor's `me` query
    // answers 401 as a matter of course; re-reading it on that answer makes it
    // retrigger itself — refetch, 401, invalidate, refetch — and the app
    // floods the API until the rate limiter cuts it off, which is how the loop
    // was found rather than noticed.
    expect(shouldRecheckSession(unauthorized, [ME])).toBe(false)
  })

  it('ignores failures that are not about being signed in', () => {
    expect(shouldRecheckSession(new ApiError('cv not found', 404), ['cvs'])).toBe(false)
    expect(shouldRecheckSession(new ApiError('too many requests', 429), ['cvs'])).toBe(false)
    // A template that failed to compile is the document's problem, not the
    // session's, and the editor handles it inline.
    expect(shouldRecheckSession(new ApiError('did not compile', 422), ['render'])).toBe(false)
    expect(shouldRecheckSession(new Error('network down'), ['cvs'])).toBe(false)
  })
})
