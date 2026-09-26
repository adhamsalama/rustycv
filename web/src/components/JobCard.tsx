import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { STATUS_COLUMNS } from '../types'
import type { Application, ApplicationInput, CvSummary } from '../types'
import { TextArea, TextField } from './Fields'

function toInput(application: Application): ApplicationInput {
  const { company, role, url, notes, status, cvId } = application
  return { company, role, url, notes, status, cvId }
}

export function JobCard({
  application,
  cvs,
  onSave,
  onDelete,
}: {
  application: Application
  cvs: CvSummary[]
  onSave: (input: ApplicationInput) => void
  onDelete: () => void
}) {
  const [draft, setDraft] = useState<ApplicationInput | null>(null)
  const editing = draft !== null

  // An open form must not be draggable: the pointer is there to type.
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: application.id,
    disabled: editing,
  })

  const style = { transform: CSS.Transform.toString(transform), transition }
  const patch = (fields: Partial<ApplicationInput>) =>
    setDraft((current) => (current ? { ...current, ...fields } : current))

  if (editing) {
    return (
      <form
        ref={setNodeRef}
        style={style}
        className="job-card editing"
        onSubmit={(e) => {
          e.preventDefault()
          onSave(draft)
          setDraft(null)
        }}
      >
        <TextField label="Company" value={draft.company} onChange={(company) => patch({ company })} />
        <TextField label="Role" value={draft.role} onChange={(role) => patch({ role })} />
        <TextField
          label="Job post"
          type="url"
          placeholder="https://…"
          value={draft.url}
          onChange={(url) => patch({ url })}
        />

        {/* The column, as a control rather than a drag. This is the whole
            keyboard route across the board, so it is not optional polish. */}
        <label className="field">
          <span className="field-label">Column</span>
          <select
            value={draft.status}
            onChange={(e) => patch({ status: e.target.value as ApplicationInput['status'] })}
          >
            {STATUS_COLUMNS.map((column) => (
              <option key={column.status} value={column.status}>
                {column.label}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span className="field-label">CV sent</span>
          <select
            value={draft.cvId ?? ''}
            onChange={(e) => patch({ cvId: e.target.value || null })}
          >
            <option value="">None yet</option>
            {cvs.map((cv) => (
              <option key={cv.id} value={cv.id}>
                {cv.title}
              </option>
            ))}
          </select>
        </label>

        <TextArea
          label="Notes"
          rows={3}
          value={draft.notes}
          placeholder="Recruiter, salary, next step…"
          onChange={(notes) => patch({ notes })}
        />

        <div className="job-card-actions">
          <button type="submit" className="primary">
            Save
          </button>
          <button type="button" className="ghost" onClick={() => setDraft(null)}>
            Cancel
          </button>
          <span className="spacer" />
          <button
            type="button"
            className="ghost danger"
            onClick={() => {
              if (confirm(`Delete ${application.company || 'this card'}?`)) onDelete()
            }}
          >
            Delete
          </button>
        </div>
      </form>
    )
  }

  return (
    <article ref={setNodeRef} style={style} className={isDragging ? 'job-card dragging' : 'job-card'}>
      <div className="job-card-head">
        <button
          type="button"
          className="drag-handle"
          aria-label={`Move ${application.company || 'card'}`}
          {...attributes}
          {...listeners}
        >
          ⠿
        </button>
        <button type="button" className="job-card-title" onClick={() => setDraft(toInput(application))}>
          <strong>{application.company || 'Untitled'}</strong>
          {application.role ? <span className="muted small">{application.role}</span> : null}
        </button>
      </div>

      {application.notes ? <p className="job-card-notes small">{application.notes}</p> : null}

      <div className="job-card-foot">
        {application.url ? (
          <a className="pill" href={application.url} target="_blank" rel="noreferrer">
            Post ↗
          </a>
        ) : null}
        {/* Null once the CV is deleted — the application still happened. */}
        {application.cvId && application.cvTitle ? (
          <Link className="pill" to={`/cv/${application.cvId}`}>
            {application.cvTitle}
          </Link>
        ) : null}
      </div>
    </article>
  )
}
