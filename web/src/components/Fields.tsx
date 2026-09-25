import type { ChangeEvent } from 'react'
import type { DateSpec } from '../types'

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']

interface TextFieldProps {
  label: string
  value: string
  onChange: (value: string) => void
  type?: string
  placeholder?: string
}

export function TextField({ label, value, onChange, type = 'text', placeholder }: TextFieldProps) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <input
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.value)}
      />
    </label>
  )
}

interface TextAreaProps {
  label: string
  value: string
  onChange: (value: string) => void
  rows?: number
  placeholder?: string
}

export function TextArea({ label, value, onChange, rows = 3, placeholder }: TextAreaProps) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <textarea
        rows={rows}
        value={value}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value)}
      />
    </label>
  )
}

export function CheckboxField({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
}) {
  return (
    <label className="field field-checkbox">
      <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
      <span>{label}</span>
    </label>
  )
}

/** A `string[]` edited as a comma-separated list. */
export function TagsField({
  label,
  value,
  onChange,
  placeholder,
}: {
  label: string
  value: string[]
  onChange: (value: string[]) => void
  placeholder?: string
}) {
  return (
    <label className="field">
      <span className="field-label">{label}</span>
      <input
        type="text"
        value={value.join(', ')}
        placeholder={placeholder}
        onChange={(e) => onChange(e.target.value.split(',').map((s) => s.trimStart()))}
        onBlur={(e) =>
          onChange(
            e.target.value
              .split(',')
              .map((s) => s.trim())
              .filter(Boolean),
          )
        }
      />
    </label>
  )
}

/**
 * Month + year, the precision a CV actually needs.
 *
 * "Any month" is a real choice, not a missing one — education entries read
 * "2017 – 2022" — so it is an explicit option rather than a blank.
 */
export function DateField({
  label,
  value,
  onChange,
}: {
  label: string
  value: DateSpec | null
  onChange: (value: DateSpec | null) => void
}) {
  const year = value?.year ?? ''
  const month = value?.month ?? ''

  const setYear = (raw: string) => {
    if (raw === '') return onChange(null)
    const parsed = Number.parseInt(raw, 10)
    if (Number.isNaN(parsed)) return
    onChange({ year: parsed, month: value?.month ?? null })
  }

  return (
    <div className="field">
      <span className="field-label">{label}</span>
      <div className="field-date">
        <select
          value={month}
          disabled={value === null}
          onChange={(e) =>
            onChange({
              year: value?.year ?? new Date().getFullYear(),
              month: e.target.value === '' ? null : Number(e.target.value),
            })
          }
        >
          <option value="">Year only</option>
          {MONTHS.map((name, index) => (
            <option key={name} value={index + 1}>
              {name}
            </option>
          ))}
        </select>
        <input
          type="number"
          inputMode="numeric"
          placeholder="Year"
          value={year}
          min={1900}
          max={2100}
          onChange={(e) => setYear(e.target.value)}
        />
      </div>
    </div>
  )
}
