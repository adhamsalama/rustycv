import { AppearanceToggle } from '../components/AppearanceToggle'
import { AuthForm } from '../components/AuthForm'

/**
 * What a signed-out visitor sees.
 *
 * The sign-in form is on the page rather than behind a button: someone with an
 * account should not have to read a pitch to get past it, and someone without
 * one can read down the page with the form still in view.
 *
 * The templates below are named and described exactly as the renderer
 * describes them, because that listing is the product — a vaguer version of it
 * would say less while taking up the same room.
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
            <h1>Your CV is data. The PDF is a render of it.</h1>
            <p className="lede">
              Write the thing once. Then change the template, the type size, the accent or the
              spacing, and the same document comes back typeset a different way — because nothing
              is baked into a file. The PDF is rebuilt from scratch every time you look at it.
            </p>
            <p className="landing-facts">
              Six templates <span aria-hidden="true">·</span> typeset with Typst{' '}
              <span aria-hidden="true">·</span> exports as JSON
            </p>
          </div>

          <div className="landing-form">
            <AuthForm />
          </div>
        </section>

        <section className="landing-section">
          <h2>Six templates, one document</h2>
          <p className="lede">
            Switching is a dropdown. The sections, the dates and the wording underneath do not
            change, so there is nothing to retype and nothing to lay out again.
          </p>

          <dl className="landing-templates">
            <div>
              <dt>Classic</dt>
              <dd>Single column, generous whitespace, ATS-friendly.</dd>
            </div>
            <div>
              <dt>FlowCV</dt>
              <dd>Centred header, tinted section bands, roles behind a hairline.</dd>
            </div>
            <div>
              <dt>Modern</dt>
              <dd>Accent-coloured headings with rules and a bolder name.</dd>
            </div>
            <div>
              <dt>Engineer</dt>
              <dd>
                Dense single column, ruled small-caps headings, no graphics — built to survive a
                parser.
              </dd>
            </div>
            <div>
              <dt>Banner</dt>
              <dd>Name and contact reversed out of a filled accent block, accent bars beside
                headings.</dd>
            </div>
            <div>
              <dt>Compact</dt>
              <dd>Tighter type and spacing, for CVs that spill onto a second page.</dd>
            </div>
          </dl>
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
        <p className="muted small">
          One binary and a SQLite file. Ten CVs and ten applications per account.
        </p>
      </footer>
    </div>
  )
}
