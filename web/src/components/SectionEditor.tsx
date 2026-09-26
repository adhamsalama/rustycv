import { useState } from 'react'
import { useCvStore } from '../store'
import { SECTION_SPECS, type FieldSpec } from '../sections'
import type { BaseItem, DateSpec, EntryOrder, Section } from '../types'
import type { RichText } from '../rich'
import { sectionItems, visibleItems } from '../types'
import { CheckboxField, DateField, TagsField, TextArea, TextField } from './Fields'
import { RichTextField } from './RichText'
import { SortableList, SortableRow } from './Sortable'

const asString = (value: unknown): string => (typeof value === 'string' ? value : '')
const asStrings = (value: unknown): string[] => (Array.isArray(value) ? (value as string[]) : [])
const asDate = (value: unknown): DateSpec | null =>
  value && typeof value === 'object' ? (value as DateSpec) : null
const asRich = (value: unknown): RichText =>
  typeof value === 'string' || Array.isArray(value) ? (value as RichText) : ''

function Field({
  spec,
  item,
  onChange,
}: {
  spec: FieldSpec
  item: BaseItem
  onChange: (patch: Record<string, unknown>) => void
}) {
  const set = (value: unknown) => onChange({ [spec.key]: value })

  switch (spec.type) {
    case 'textarea':
      return (
        <TextArea
          label={spec.label}
          value={asString(item[spec.key])}
          rows={spec.rows}
          placeholder={spec.placeholder}
          onChange={set}
        />
      )
    case 'checkbox':
      return (
        <CheckboxField label={spec.label} checked={item[spec.key] === true} onChange={set} />
      )
    case 'date':
      return <DateField label={spec.label} value={asDate(item[spec.key])} onChange={set} />
    case 'rich':
      return (
        <RichTextField
          label={spec.label}
          value={asRich(item[spec.key])}
          placeholder={spec.placeholder}
          emptyAs={spec.startsAsList ? 'bullet' : 'paragraph'}
          onChange={set}
        />
      )
    case 'tags':
      return (
        <TagsField
          label={spec.label}
          value={asStrings(item[spec.key])}
          placeholder={spec.placeholder}
          onChange={set}
        />
      )
    default:
      return (
        <TextField
          label={spec.label}
          type={spec.type}
          value={asString(item[spec.key])}
          placeholder={spec.placeholder}
          onChange={set}
        />
      )
  }
}

/**
 * Switches that belong to a section rather than to any one entry.
 *
 * Only work experience has any today, so this renders nothing elsewhere rather
 * than inventing a general mechanism for a single case.
 */
function SectionOptions({ section }: { section: Section }) {
  const patch = useCvStore((s) => s.patchSectionOptions)
  if (section.kind !== 'experience') return null

  return (
    <div className="section-options">
      <label className="field">
        <span className="field-label">Title / subtitle order</span>
        <select
          value={section.order}
          onChange={(e) => patch(section.id, { order: e.target.value as EntryOrder })}
        >
          <option value="roleFirst">Job title – Employer</option>
          <option value="companyFirst">Employer – Job title</option>
        </select>
      </label>

      <label className="field field-checkbox">
        <input
          type="checkbox"
          checked={section.groupPromotions}
          onChange={(e) => patch(section.id, { groupPromotions: e.target.checked })}
        />
        <span>
          Group promotions
          <span className="muted small">
            {' '}
            — consecutive roles at one employer share a heading
          </span>
        </span>
      </label>
    </div>
  )
}

function ItemCard({
  section,
  item,
  index,
  open,
  onToggle,
}: {
  section: Section
  item: BaseItem
  index: number
  open: boolean
  onToggle: () => void
}) {
  const spec = SECTION_SPECS[section.kind]
  const updateItem = useCvStore((s) => s.updateItem)
  const removeItem = useCvStore((s) => s.removeItem)
  const toggleItem = useCvStore((s) => s.toggleItem)
  const hidden = item.visible === false

  return (
    <SortableRow id={item.id}>
      {(handleProps) => (
        <div className={hidden ? 'item-card hidden' : 'item-card'}>
          <div className="item-head">
            <button
              type="button"
              className="drag-handle"
              aria-label={`Reorder ${spec.itemLabel} ${index + 1}`}
              {...handleProps}
            >
              ⠿
            </button>
            <button type="button" className="item-title" onClick={onToggle}>
              {spec.summary(item)}
              {hidden ? <span className="pill">Hidden</span> : null}
            </button>
            <button
              type="button"
              className="ghost"
              aria-pressed={hidden}
              title={hidden ? 'Include this entry in the PDF' : 'Keep this entry but leave it out of the PDF'}
              onClick={() => toggleItem(section.id, item.id)}
            >
              {hidden ? 'Show' : 'Hide'}
            </button>
            <button
              type="button"
              className="ghost danger"
              onClick={() => removeItem(section.id, item.id)}
            >
              Remove
            </button>
          </div>

          {open ? (
            <div className="item-fields">
              {spec.fields.map((field) => (
                <div
                  key={field.key}
                  className={'width' in field && field.width === 'half' ? 'half' : 'full'}
                >
                  <Field
                    spec={field}
                    item={item}
                    onChange={(patch) => updateItem(section.id, item.id, patch)}
                  />
                </div>
              ))}
            </div>
          ) : null}
        </div>
      )}
    </SortableRow>
  )
}

export function SectionEditor({ section }: { section: Section }) {
  const spec = SECTION_SPECS[section.kind]
  const items = sectionItems(section)
  const visible = visibleItems(section)
  const { addItem, moveItem, renameSection, toggleSection, removeSection } = useCvStore()

  // Newly added entries open themselves; everything else starts collapsed so a
  // long CV stays scannable.
  const [openId, setOpenId] = useState<string | null>(items[0]?.id ?? null)

  return (
    <section className="section-editor">
      <header className="section-head">
        <input
          className="section-title"
          value={section.title}
          onChange={(e) => renameSection(section.id, e.target.value)}
          aria-label="Section title"
        />
        <div className="section-actions">
          <button type="button" className="ghost" onClick={() => toggleSection(section.id)}>
            {section.visible ? 'Hide' : 'Show'}
          </button>
          <button type="button" className="ghost danger" onClick={() => removeSection(section.id)}>
            Delete section
          </button>
        </div>
      </header>

      {!section.visible ? (
        <p className="muted">This section is hidden and will not appear in the PDF.</p>
      ) : null}

      <SectionOptions section={section} />

      <SortableList
        ids={items.map((i) => i.id)}
        onReorder={(from, to) => moveItem(section.id, from, to)}
      >
        <div className="item-list">
          {items.map((item, index) => (
            <ItemCard
              key={item.id}
              section={section}
              item={item}
              index={index}
              open={openId === item.id}
              onToggle={() => setOpenId(openId === item.id ? null : item.id)}
            />
          ))}
        </div>
      </SortableList>

      {items.length === 0 ? <p className="muted">Nothing here yet.</p> : null}
      {items.length > 0 && visible.length === 0 ? (
        <p className="muted">
          Every entry here is hidden, so this section will not appear in the PDF.
        </p>
      ) : null}

      <button
        type="button"
        className="secondary"
        onClick={() => {
          addItem(section.id)
          // Open whatever was just appended.
          const next = useCvStore.getState().document?.sections.find((s) => s.id === section.id)
          const added = next ? sectionItems(next).at(-1) : undefined
          if (added) setOpenId(added.id)
        }}
      >
        Add {spec.itemLabel}
      </button>
    </section>
  )
}
