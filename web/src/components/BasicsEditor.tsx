import { useCvStore } from '../store'
import { TextArea, TextField } from './Fields'

export function BasicsEditor() {
  const basics = useCvStore((s) => s.document?.basics)
  const { patchBasics, addLink, updateLink, removeLink } = useCvStore()

  if (!basics) return null

  return (
    <section className="section-editor">
      <header className="section-head">
        <h2 className="section-title-static">Details</h2>
      </header>

      <div className="item-fields">
        <div className="half">
          <TextField
            label="Full name"
            value={basics.fullName}
            onChange={(fullName) => patchBasics({ fullName })}
          />
        </div>
        <div className="half">
          <TextField
            label="Headline"
            value={basics.headline}
            placeholder="Staff Backend Engineer"
            onChange={(headline) => patchBasics({ headline })}
          />
        </div>
        <div className="half">
          <TextField
            label="Email"
            type="email"
            value={basics.email}
            onChange={(email) => patchBasics({ email })}
          />
        </div>
        <div className="half">
          <TextField
            label="Phone"
            type="tel"
            value={basics.phone}
            onChange={(phone) => patchBasics({ phone })}
          />
        </div>
        <div className="half">
          <TextField
            label="Location"
            value={basics.location}
            onChange={(location) => patchBasics({ location })}
          />
        </div>
        <div className="full">
          <TextArea
            label="Summary"
            value={basics.summary}
            rows={3}
            placeholder="Optional. A short paragraph under your contact details."
            onChange={(summary) => patchBasics({ summary })}
          />
        </div>
      </div>

      <h3 className="subhead">Links</h3>
      <div className="link-list">
        {basics.links.map((link) => (
          <div className="link-row" key={link.id}>
            <TextField
              label="Label"
              value={link.label}
              placeholder="GitHub"
              onChange={(label) => updateLink(link.id, { label })}
            />
            <TextField
              label="URL"
              type="url"
              value={link.url}
              placeholder="https://github.com/you"
              onChange={(url) => updateLink(link.id, { url })}
            />
            <button type="button" className="ghost danger" onClick={() => removeLink(link.id)}>
              Remove
            </button>
          </div>
        ))}
      </div>
      <button type="button" className="secondary" onClick={addLink}>
        Add link
      </button>
    </section>
  )
}
