import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
  DndContext,
  KeyboardSensor,
  PointerSensor,
  closestCorners,
  useDroppable,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragOverEvent,
} from '@dnd-kit/core'
import {
  SortableContext,
  sortableKeyboardCoordinates,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable'
import { api } from '../api'
import { applyMove, columnId, dropTarget, groupByStatus } from '../board'
import { STATUS_COLUMNS } from '../types'
import type { Application, ApplicationInput, ApplicationStatus, CvSummary } from '../types'
import { AccountMenu } from '../components/AccountMenu'
import { AppearanceToggle } from '../components/AppearanceToggle'
import { JobCard } from '../components/JobCard'
import { SiteFooter } from '../components/SiteFooter'

const QUERY_KEY = ['applications']

/**
 * Where the CVs go. The board is the other half of "one CV per application":
 * a card records which CV was sent, so the tailored duplicate is findable
 * later by the company it was tailored for rather than by its title.
 */
export function Board() {
  const queryClient = useQueryClient()
  const [overStatus, setOverStatus] = useState<ApplicationStatus | null>(null)
  const [error, setError] = useState<string | null>(null)

  const { data: applications = [], isLoading } = useQuery({
    queryKey: QUERY_KEY,
    queryFn: api.listApplications,
  })
  const { data: cvs = [] } = useQuery({ queryKey: ['cvs'], queryFn: api.listCvs })

  const refresh = () => queryClient.invalidateQueries({ queryKey: QUERY_KEY })

  const onError = (failure: Error) => setError(failure.message)

  const create = useMutation({
    mutationFn: api.createApplication,
    onSuccess: () => {
      setError(null)
      return refresh()
    },
    onError,
  })
  const save = useMutation({
    mutationFn: ({ id, input }: { id: string; input: ApplicationInput }) =>
      api.updateApplication(id, input),
    onSuccess: refresh,
    onError,
  })
  const remove = useMutation({ mutationFn: api.deleteApplication, onSuccess: refresh, onError })

  /**
   * A drop is applied to the cache first and confirmed afterwards. Waiting for
   * the round trip would let the card snap back to where it was for a frame,
   * which reads as a failed drag; `applyMove` is the same arithmetic the
   * server does, so the confirmation changes nothing.
   */
  const move = useMutation({
    mutationFn: ({ id, status, index }: { id: string; status: ApplicationStatus; index: number }) =>
      api.moveApplication(id, status, index),
    onMutate: async ({ id, status, index }) => {
      await queryClient.cancelQueries({ queryKey: QUERY_KEY })
      const previous = queryClient.getQueryData<Application[]>(QUERY_KEY)
      if (previous) queryClient.setQueryData(QUERY_KEY, applyMove(previous, id, status, index))
      return { previous }
    },
    onError: (_error, _variables, context) => {
      if (context?.previous) queryClient.setQueryData(QUERY_KEY, context.previous)
    },
    onSettled: refresh,
  })

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 4 } }),
    useSensor(KeyboardSensor, { coordinateGetter: sortableKeyboardCoordinates }),
  )

  const columns = groupByStatus(applications)

  const hovered = (event: DragOverEvent | DragEndEvent) => {
    const activeId = String(event.active.id)
    const target = dropTarget(applications, activeId, event.over ? String(event.over.id) : null)
    // Over itself resolves to no move; the card is still in its own column.
    return target ?? { status: applications.find((a) => a.id === activeId)?.status ?? null, index: -1 }
  }

  return (
    <main className="board-page">
      <header className="dashboard-head">
        <div>
          <h1>Job tracker</h1>
          <p className="muted">
            Where each application got to, and which CV you sent. Drag a card between columns, or
            change its column from the card itself.
          </p>
        </div>
        <div className="dashboard-actions">
          <AccountMenu />
          <AppearanceToggle />
          <Link className="secondary" to="/">
            CVs
          </Link>
        </div>
      </header>

      {/* Where a refused card lands — the 10-application cap, or a card
          pointing at a CV that is no longer there. */}
      {error ? (
        <p className="badge badge-error" role="alert">
          {error}
        </p>
      ) : null}

      {isLoading ? <p className="muted">Loading…</p> : null}

      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragOver={(event) => setOverStatus(hovered(event).status)}
        onDragCancel={() => setOverStatus(null)}
        onDragEnd={(event) => {
          setOverStatus(null)
          const target = dropTarget(
            applications,
            String(event.active.id),
            event.over ? String(event.over.id) : null,
          )
          if (target) move.mutate({ id: String(event.active.id), ...target })
        }}
      >
        <div className="board">
          {STATUS_COLUMNS.map((column) => (
            <Column
              key={column.status}
              status={column.status}
              label={column.label}
              applications={columns[column.status]}
              cvs={cvs}
              isTarget={overStatus === column.status}
              onAdd={(input) => create.mutate({ ...input, status: column.status })}
              onSave={(id, input) => save.mutate({ id, input })}
              onDelete={(id) => remove.mutate(id)}
            />
          ))}
        </div>
      </DndContext>

      <footer className="app-foot">
        <SiteFooter />
      </footer>
    </main>
  )
}

function Column({
  status,
  label,
  applications,
  cvs,
  isTarget,
  onAdd,
  onSave,
  onDelete,
}: {
  status: ApplicationStatus
  label: string
  applications: Application[]
  cvs: CvSummary[]
  isTarget: boolean
  onAdd: (input: Partial<ApplicationInput>) => void
  onSave: (id: string, input: ApplicationInput) => void
  onDelete: (id: string) => void
}) {
  // The body, not the whole column, so the gap under the last card is a drop
  // target and the "Add" control below it is not.
  const { setNodeRef } = useDroppable({ id: columnId(status) })

  return (
    <section className={isTarget ? 'board-column target' : 'board-column'}>
      <header className="board-column-head">
        <h2>{label}</h2>
        <span className="count">{applications.length}</span>
      </header>

      <SortableContext items={applications.map((a) => a.id)} strategy={verticalListSortingStrategy}>
        <div ref={setNodeRef} className="board-column-body">
          {applications.map((application) => (
            <JobCard
              key={application.id}
              application={application}
              cvs={cvs}
              onSave={(input) => onSave(application.id, input)}
              onDelete={() => onDelete(application.id)}
            />
          ))}
        </div>
      </SortableContext>

      <AddCard onAdd={onAdd} />
    </section>
  )
}

/** Two fields and Enter. Everything else about a card is edited on the card. */
function AddCard({ onAdd }: { onAdd: (input: Partial<ApplicationInput>) => void }) {
  const [open, setOpen] = useState(false)
  const [company, setCompany] = useState('')
  const [role, setRole] = useState('')

  if (!open) {
    return (
      <button type="button" className="ghost board-add" onClick={() => setOpen(true)}>
        + Add
      </button>
    )
  }

  return (
    <form
      className="board-add-form"
      onSubmit={(e) => {
        e.preventDefault()
        if (!company.trim() && !role.trim()) return setOpen(false)
        onAdd({ company, role })
        setCompany('')
        setRole('')
        // Stays open: applications arrive in batches.
      }}
    >
      <input
        autoFocus
        type="text"
        placeholder="Company"
        value={company}
        onChange={(e) => setCompany(e.target.value)}
      />
      <input
        type="text"
        placeholder="Role"
        value={role}
        onChange={(e) => setRole(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape') setOpen(false)
        }}
      />
      <div className="board-add-actions">
        <button type="submit" className="primary">
          Add
        </button>
        <button type="button" className="ghost" onClick={() => setOpen(false)}>
          Done
        </button>
      </div>
    </form>
  )
}
