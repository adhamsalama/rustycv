import { describe, expect, it } from 'vitest'
import { applyMove, columnId, dropTarget, groupByStatus } from './board'
import type { Application, ApplicationStatus } from './types'

function card(id: string, status: ApplicationStatus): Application {
  return {
    id,
    company: id.toUpperCase(),
    role: 'Engineer',
    url: '',
    notes: '',
    status,
    cvId: null,
    cvTitle: null,
    createdAt: '2026-01-01T00:00:00Z',
    updatedAt: '2026-01-01T00:00:00Z',
  }
}

/**
 * [a, b, c] in Wishlist; [d] in Applied; the rest empty.
 *
 * Built fresh per call rather than shared: a mutating `applyMove` would
 * otherwise scribble on the fixture, and the test that checks for exactly that
 * would pass because an earlier test had already made the same change.
 */
const board = () => [
  card('a', 'wishlist'),
  card('b', 'wishlist'),
  card('c', 'wishlist'),
  card('d', 'applied'),
]

const ids = (applications: Application[], status: ApplicationStatus) =>
  groupByStatus(applications)[status].map((a) => a.id)

describe('grouping', () => {
  it('keeps every column, including the empty ones', () => {
    const columns = groupByStatus(board())
    expect(columns.wishlist.map((a) => a.id)).toEqual(['a', 'b', 'c'])
    expect(columns.interview).toEqual([])
  })
})

describe('a drop', () => {
  it('takes the place of the card it was dropped on, dragging down', () => {
    // a over c: the two it passed shift up, and it lands last.
    const target = dropTarget(board(), 'a', 'c')
    expect(target).toEqual({ status: 'wishlist', index: 2 })
    expect(ids(applyMove(board(), 'a', 'wishlist', 2), 'wishlist')).toEqual(['b', 'c', 'a'])
  })

  it('takes the place of the card it was dropped on, dragging up', () => {
    const target = dropTarget(board(), 'c', 'a')
    expect(target).toEqual({ status: 'wishlist', index: 0 })
    expect(ids(applyMove(board(), 'c', 'wishlist', 0), 'wishlist')).toEqual(['c', 'a', 'b'])
  })

  it('lands above the card it was dropped on in another column', () => {
    const target = dropTarget(board(), 'a', 'd')
    expect(target).toEqual({ status: 'applied', index: 0 })

    const moved = applyMove(board(), 'a', 'applied', 0)
    expect(ids(moved, 'applied')).toEqual(['a', 'd'])
    expect(ids(moved, 'wishlist')).toEqual(['b', 'c'])
  })

  it('goes to the foot of the column when dropped on the column itself', () => {
    expect(dropTarget(board(), 'a', columnId('applied'))).toEqual({ status: 'applied', index: 1 })
    // Back into its own column: its current slot must not count twice.
    expect(dropTarget(board(), 'a', columnId('wishlist'))).toEqual({ status: 'wishlist', index: 2 })
    expect(dropTarget(board(), 'a', columnId('interview'))).toEqual({ status: 'interview', index: 0 })
  })

  it('is nothing at all when a card is dropped on itself or on no target', () => {
    expect(dropTarget(board(), 'a', 'a')).toBeNull()
    expect(dropTarget(board(), 'a', null)).toBeNull()
  })
})

describe('applying a move', () => {
  it('clamps an index past the end of the column', () => {
    expect(ids(applyMove(board(), 'a', 'offer', 99), 'offer')).toEqual(['a'])
  })

  it('leaves a board it cannot find the card in alone', () => {
    const cards = board()
    expect(applyMove(cards, 'gone', 'offer', 0)).toBe(cards)
  })

  it('does not mutate the list it was given', () => {
    // React Query hands it the cached list: mutating that in place would move
    // the card in the cache without re-rendering, and the board would only
    // catch up on the next refetch.
    const cards = board()
    const before = cards.map((a) => `${a.id}:${a.status}`)
    applyMove(cards, 'a', 'offer', 0)
    expect(cards.map((a) => `${a.id}:${a.status}`)).toEqual(before)
  })
})
