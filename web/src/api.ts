import type {
  Application,
  ApplicationInput,
  ApplicationStatus,
  Cv,
  CvDocument,
  CvSummary,
  Diagnostic,
  TemplateInfo,
  User,
} from './types'

/** An API error that carries the server's Typst diagnostics when it has them. */
export class ApiError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly diagnostics: Diagnostic[] = [],
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api${path}`, {
    ...init,
    headers: init?.body ? { 'content-type': 'application/json', ...init?.headers } : init?.headers,
  })

  if (!response.ok) {
    throw await toApiError(response)
  }
  if (response.status === 204) {
    return undefined as T
  }
  return response.json() as Promise<T>
}

async function toApiError(response: Response): Promise<ApiError> {
  try {
    const body = await response.json()
    return new ApiError(body.error ?? response.statusText, response.status, body.diagnostics ?? [])
  } catch {
    return new ApiError(response.statusText, response.status)
  }
}

export const api = {
  /**
   * Who is signed in. A 401 is the normal answer for a signed-out visitor, not
   * a failure — `AuthGate` turns it into the login form.
   */
  me: () => request<User>('/auth/me'),
  signup: (email: string, password: string) =>
    request<User>('/auth/signup', { method: 'POST', body: JSON.stringify({ email, password }) }),
  login: (email: string, password: string) =>
    request<User>('/auth/login', { method: 'POST', body: JSON.stringify({ email, password }) }),
  logout: () => request<void>('/auth/logout', { method: 'POST', body: '{}' }),

  templates: () => request<TemplateInfo[]>('/templates'),
  fonts: () => request<string[]>('/fonts'),

  listCvs: () => request<CvSummary[]>('/cvs'),
  getCv: (id: string) => request<Cv>(`/cvs/${id}`),
  createCv: (title?: string) =>
    request<Cv>('/cvs', { method: 'POST', body: JSON.stringify({ title }) }),
  duplicateCv: (id: string) =>
    request<Cv>(`/cvs?from=${encodeURIComponent(id)}`, { method: 'POST', body: '{}' }),
  saveCv: (id: string, document: CvDocument, title?: string) =>
    request<Cv>(`/cvs/${id}`, { method: 'PUT', body: JSON.stringify({ title, document }) }),
  deleteCv: (id: string) => request<void>(`/cvs/${id}`, { method: 'DELETE' }),
  importCv: (document: CvDocument, title?: string) =>
    request<Cv>('/cvs/import', { method: 'POST', body: JSON.stringify({ title, document }) }),

  listApplications: () => request<Application[]>('/applications'),
  createApplication: (input: Partial<ApplicationInput>) =>
    request<Application>('/applications', { method: 'POST', body: JSON.stringify(input) }),
  updateApplication: (id: string, input: ApplicationInput) =>
    request<Application>(`/applications/${id}`, { method: 'PUT', body: JSON.stringify(input) }),
  /** Where a drag ended: which column, and where in it. */
  moveApplication: (id: string, status: ApplicationStatus, index: number) =>
    request<Application>(`/applications/${id}/move`, {
      method: 'POST',
      body: JSON.stringify({ status, index }),
    }),
  deleteApplication: (id: string) => request<void>(`/applications/${id}`, { method: 'DELETE' }),

  /**
   * Render whatever is currently in the editor, saved or not.
   *
   * Takes a signal so a superseded keystroke's render can be abandoned instead
   * of racing the one after it.
   */
  async renderPdf(document: CvDocument, signal?: AbortSignal): Promise<Blob> {
    const response = await fetch('/api/render', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(document),
      signal,
    })
    if (!response.ok) {
      throw await toApiError(response)
    }
    return response.blob()
  },

  pdfUrl: (id: string) => `/api/cvs/${id}/pdf`,
  exportUrl: (id: string) => `/api/cvs/${id}/export`,
}
