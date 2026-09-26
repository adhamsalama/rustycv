// The board's arithmetic, kept out of the component so it can be tested
// without a DOM and a drag.
//
// The server holds the same two rules — a card is removed from the list and
// re-inserted at `index` of its destination column — so `applyMove` is what
// the optimistic update and the eventual reload both produce. If they ever
// disagree, a dropped card visibly jumps when the refetch lands.

import type { Application, ApplicationStatus } from './types'
import { STATUS_COLUMNS } from './types'

/** A column's droppable id, distinct from any card's UUID. */
export function columnId(status: ApplicationStatus): string {
  return `column:${status}`
}

function statusOfColumnId(id: string): ApplicationStatus | null {
  const status = id.startsWith('column:') ? id.slice('column:'.length) : null
  return STATUS_COLUMNS.some((c) => c.status === status)
    ? (status as ApplicationStatus)
    : null
}

export type Board = Record<ApplicationStatus, Application[]>

/** The flat list the API returns, split into columns and keeping its order. */
export function groupByStatus(applications: Application[]): Board {
  const board = Object.fromEntries(
    STATUS_COLUMNS.map((c) => [c.status, [] as Application[]]),
  ) as Board
  for (const application of applications) {
    // A status the client doesn't know would otherwise vanish silently; there
    // is no column to draw it in, so it is skipped rather than guessed at.
    board[application.status]?.push(application)
  }
  return board
}

/**
 * Where a drag ends, as the move endpoint wants it.
 *
 * Dropped on a card: take that card's place, which is `arrayMove` semantics
 * within one column and "insert above it" across two. Dropped on the column
 * itself — the empty space below the cards — means last.
 */
export function dropTarget(
  applications: Application[],
  activeId: string,
  overId: string | null,
): { status: ApplicationStatus; index: number } | null {
  if (!overId || overId === activeId) return null
  const board = groupByStatus(applications)

  const column = statusOfColumnId(overId)
  if (column) {
    return { status: column, index: board[column].filter((a) => a.id !== activeId).length }
  }

  const over = applications.find((a) => a.id === overId)
  if (!over) return null
  return { status: over.status, index: board[over.status].findIndex((a) => a.id === overId) }
}

/** The board as it will be once the server has applied that move. */
export function applyMove(
  applications: Application[],
  id: string,
  status: ApplicationStatus,
  index: number,
): Application[] {
  const moving = applications.find((a) => a.id === id)
  if (!moving) return applications

  const board = groupByStatus(applications.filter((a) => a.id !== id))
  const column = board[status]
  if (!column) return applications

  column.splice(Math.min(index, column.length), 0, { ...moving, status })
  return STATUS_COLUMNS.flatMap((c) => board[c.status])
}
