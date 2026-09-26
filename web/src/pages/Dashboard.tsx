import { useRef, useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { api } from '../api'
import { AccountMenu } from '../components/AccountMenu'
import { AppearanceToggle } from '../components/AppearanceToggle'
import type { CvDocument } from '../types'

export function Dashboard() {
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const fileInput = useRef<HTMLInputElement>(null)
  /** Whatever last stopped a CV being made: a bad import, or the 10-CV cap. */
  const [error, setError] = useState<string | null>(null)

  const { data: cvs = [], isLoading } = useQuery({ queryKey: ['cvs'], queryFn: api.listCvs })

  const refresh = () => queryClient.invalidateQueries({ queryKey: ['cvs'] })

  const onError = (failure: Error) => setError(failure.message)

  const create = useMutation({
    mutationFn: () => api.createCv(),
    onSuccess: (cv) => navigate(`/cv/${cv.id}`),
    onError,
  })
  const duplicate = useMutation({ mutationFn: api.duplicateCv, onSuccess: refresh, onError })
  const remove = useMutation({ mutationFn: api.deleteCv, onSuccess: refresh, onError })
  const importCv = useMutation({
    mutationFn: (document: CvDocument) => api.importCv(document),
    onSuccess: (cv) => navigate(`/cv/${cv.id}`),
    onError,
  })

  const handleFile = async (file: File) => {
    setError(null)
    try {
      importCv.mutate(JSON.parse(await file.text()) as CvDocument)
    } catch {
      setError('That file is not valid JSON.')
    }
  }

  return (
    <main className="dashboard">
      <header className="dashboard-head">
        <div>
          <h1>RustyCV</h1>
          <p className="muted">
            Your CVs are stored as data, not PDFs — restyle them any time, and the PDF is rebuilt
            from scratch.
          </p>
        </div>
        <div className="dashboard-actions">
          <AccountMenu />
          <AppearanceToggle />
          <Link className="ghost" to="/jobs">
            Job tracker
          </Link>
          <button type="button" className="ghost" onClick={() => fileInput.current?.click()}>
            Import JSON
          </button>
          <button
            type="button"
            className="primary"
            onClick={() => {
              setError(null)
              create.mutate()
            }}
          >
            New CV
          </button>
        </div>
      </header>

      <input
        ref={fileInput}
        type="file"
        accept="application/json,.json"
        hidden
        onChange={(e) => {
          const file = e.target.files?.[0]
          if (file) void handleFile(file)
          e.target.value = ''
        }}
      />

      {error ? (
        <p className="badge badge-error" role="alert">
          {error}
        </p>
      ) : null}

      {isLoading ? <p className="muted">Loading…</p> : null}

      {!isLoading && cvs.length === 0 ? (
        <p className="muted">No CVs yet. Create one, or import a JSON export.</p>
      ) : null}

      <ul className="cv-grid">
        {cvs.map((cv) => (
          <li key={cv.id} className="cv-card">
            <Link to={`/cv/${cv.id}`} className="cv-card-main">
              <h2>{cv.title}</h2>
              <p className="muted">{cv.fullName || 'Unnamed'}</p>
              <p className="muted small">
                {cv.template} · edited {new Date(cv.updatedAt).toLocaleDateString()}
              </p>
            </Link>
            <div className="cv-card-actions">
              <a className="ghost" href={api.pdfUrl(cv.id)}>
                PDF
              </a>
              <button
                type="button"
                className="ghost"
                onClick={() => {
                  setError(null)
                  duplicate.mutate(cv.id)
                }}
              >
                Duplicate
              </button>
              <button
                type="button"
                className="ghost danger"
                onClick={() => {
                  if (confirm(`Delete “${cv.title}”? This cannot be undone.`)) {
                    remove.mutate(cv.id)
                  }
                }}
              >
                Delete
              </button>
            </div>
          </li>
        ))}
      </ul>
    </main>
  )
}
