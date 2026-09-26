import { useParams } from 'react-router-dom'
import { useQuery } from '@tanstack/react-query'
import { api, ApiError } from '../api'
import { AppearanceToggle } from '../components/AppearanceToggle'
import { SiteFooter } from '../components/SiteFooter'

/**
 * What a share link opens to. Outside `AuthGate` in `main.tsx` on purpose —
 * whoever holds the link is never signed in, and never needs to be.
 *
 * The PDF is embedded rather than fetched and re-rendered here: it is the
 * exact same render path the owner's own preview and download use, just
 * behind a public, rate-limited route instead of an authenticated one.
 */
export function Share() {
  const { publicId = '' } = useParams()

  const { data, isLoading, error } = useQuery({
    queryKey: ['share', publicId],
    queryFn: () => api.getPublishedCv(publicId),
    enabled: Boolean(publicId),
    retry: (count, error) => !(error instanceof ApiError && error.status === 404) && count < 1,
  })

  if (isLoading) return <main className="centered">Loading…</main>

  if (error) {
    const notFound = error instanceof ApiError && error.status === 404
    return (
      <main className="centered">
        {notFound ? 'This CV is not published, or the link is wrong.' : 'Could not load this CV.'}
      </main>
    )
  }

  return (
    <div className="share-page">
      <header className="landing-bar">
        <span className="wordmark">RustyCV</span>
        <AppearanceToggle />
      </header>
      <main className="share-body">
        <div className="share-heading">
          <h1>{data?.fullName || data?.title}</h1>
          <a className="primary" href={api.publicPdfUrl(publicId)}>
            Download PDF
          </a>
        </div>
        <embed
          className="share-embed"
          src={api.publicPdfUrl(publicId)}
          type="application/pdf"
          title={data?.title}
        />
        <footer className="share-foot">
          <SiteFooter />
        </footer>
      </main>
    </div>
  )
}
