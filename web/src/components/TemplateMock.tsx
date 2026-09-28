/**
 * A drawing of a template, not a render of one.
 *
 * The landing page is shown to people without an account, and rendering real
 * pages for them would mean either the server compiling on an anonymous
 * request or a 30MB module download — both a lot for a thumbnail. So each
 * template gets a sketch in CSS: grey bars where the text goes, arranged the
 * way that template arranges it. One set of markup serves all of them; the
 * `mock-<id>` class decides which parts show and where they sit.
 */
export function TemplateMock({ id, className }: { id: string; className?: string }) {
  return (
    <div className={`mock mock-${id}${className ? ` ${className}` : ''}`} aria-hidden="true">
      <div className="mock-head">
        <i className="mock-name" />
        <i className="mock-line mock-contact" />
      </div>
      <div className="mock-body">
        <div className="mock-rail">
          <Section lines={[70, 55, 62]} />
          <Section lines={[60, 48]} />
        </div>
        <div className="mock-main">
          <Section lines={[92, 84, 88, 60]} entry />
          <Section lines={[90, 76, 40]} entry />
          <Section lines={[80, 66]} />
        </div>
      </div>
    </div>
  )
}

function Section({ lines, entry = false }: { lines: number[]; entry?: boolean }) {
  return (
    <div className="mock-sec">
      <i className="mock-h" />
      {entry && (
        <div className="mock-entry">
          <i className="mock-dot" />
          <i className="mock-title" />
          <i className="mock-date" />
        </div>
      )}
      {lines.map((width, i) => (
        <i key={i} className="mock-line" style={{ width: `${width}%` }} />
      ))}
    </div>
  )
}
