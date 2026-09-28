import { AppearanceToggle } from '../components/AppearanceToggle'
import { AuthForm } from '../components/AuthForm'
import { SiteFooter } from '../components/SiteFooter'
import { TemplateMock } from '../components/TemplateMock'

/** Fanned behind the hero, back to front: the same page three ways. */
const HERO = ['classic', 'sidebar', 'banner']

/**
 * What a signed-out visitor sees.
 *
 * The sign-in form is on the page rather than behind a button: someone with an
 * account should not have to read a pitch to get past it, and someone without
 * one can read down the page with the form still in view.
 *
 * Beside the pitch, three drawings of one CV in three templates: the claim in
 * the headline, shown rather than listed.
 */
export function Landing() {
  return (
    <div className="landing">
      <header className="landing-bar">
        <span className="wordmark">RustyCV</span>
        <AppearanceToggle />
      </header>

      <main>
        <section className="landing-hero">
          <div className="landing-pitch">
            <h1>Take the rust off your CV.</h1>
            <p className="lede">
              Write it once, then make it look new whenever you like. Pick a template, nudge the
              type size, the accent or the spacing, and the same CV comes back freshly typeset —
              nothing to retype, nothing to lay out again.
            </p>
            <p className="landing-facts">
              Typeset with Typst <span aria-hidden="true">·</span> exports as JSON
            </p>
          </div>

          <div className="landing-papers" aria-hidden="true">
            {HERO.map((id) => (
              <TemplateMock key={id} id={id} />
            ))}
          </div>

          <div className="landing-form">
            <AuthForm />
          </div>
        </section>

        <section className="landing-section landing-notes">
          <article>
            <h3>What you preview is what you download</h3>
            <p>
              The preview and the download go through the same renderer. There is no faster,
              approximate preview to quietly disagree with the file you send.
            </p>
          </article>
          <article>
            <h3>The tracker sits beside the CVs</h3>
            <p>
              An application records the company, the stage it reached, and which CV you sent it.
              Delete that CV later and the card stays — having applied outlives the file.
            </p>
          </article>
          <article>
            <h3>Nothing is locked in</h3>
            <p>
              Any CV exports as JSON and imports back. The document is the part worth keeping: a
              PDF can always be made from it, and never the other way round.
            </p>
          </article>
        </section>
      </main>

      <footer className="landing-foot">
        <SiteFooter />
      </footer>
    </div>
  )
}
