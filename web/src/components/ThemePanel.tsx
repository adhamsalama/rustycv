import { useQuery } from '@tanstack/react-query'
import { api } from '../api'
import { useCvStore } from '../store'
import type { HeadingStyle, PageSize } from '../types'

const HEADING_STYLES: { value: HeadingStyle; label: string }[] = [
  { value: 'bold', label: 'Bold' },
  { value: 'underline', label: 'Bold + rule' },
  { value: 'caps', label: 'Uppercase' },
]

export function ThemePanel() {
  const document = useCvStore((s) => s.document)
  const { patchTheme, setTemplate } = useCvStore()

  const { data: templates = [] } = useQuery({ queryKey: ['templates'], queryFn: api.templates })
  const { data: fonts = [] } = useQuery({ queryKey: ['fonts'], queryFn: api.fonts })

  if (!document) return null
  const { theme } = document

  return (
    <section className="section-editor">
      <header className="section-head">
        <h2 className="section-title-static">Design</h2>
      </header>

      <div className="item-fields">
        <div className="half">
          <label className="field">
            <span className="field-label">Template</span>
            <select value={document.template} onChange={(e) => setTemplate(e.target.value)}>
              {templates.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="half">
          <label className="field">
            <span className="field-label">Font</span>
            <select
              value={theme.fontFamily}
              onChange={(e) => patchTheme({ fontFamily: e.target.value })}
            >
              {fonts.map((font) => (
                <option key={font} value={font}>
                  {font}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="half">
          <label className="field">
            <span className="field-label">Accent</span>
            <input
              type="color"
              value={theme.accent}
              onChange={(e) => patchTheme({ accent: e.target.value })}
            />
          </label>
        </div>

        <div className="half">
          <label className="field">
            <span className="field-label">Headings</span>
            <select
              value={theme.headingStyle}
              onChange={(e) => patchTheme({ headingStyle: e.target.value as HeadingStyle })}
            >
              {HEADING_STYLES.map((s) => (
                <option key={s.value} value={s.value}>
                  {s.label}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div className="half">
          <label className="field">
            <span className="field-label">Page</span>
            <select
              value={theme.page}
              onChange={(e) => patchTheme({ page: e.target.value as PageSize })}
            >
              <option value="a4">A4</option>
              <option value="letter">US Letter</option>
            </select>
          </label>
        </div>

        <Slider
          label="Text size"
          value={theme.fontSizePt}
          min={8}
          max={13}
          step={0.25}
          suffix="pt"
          onChange={(fontSizePt) => patchTheme({ fontSizePt })}
        />
        <Slider
          label="Margins"
          value={theme.marginMm}
          min={8}
          max={30}
          step={1}
          suffix="mm"
          onChange={(marginMm) => patchTheme({ marginMm })}
        />
        <Slider
          label="Line height"
          value={theme.lineHeight}
          min={0.85}
          max={1.5}
          step={0.05}
          suffix="×"
          onChange={(lineHeight) => patchTheme({ lineHeight })}
        />
        <Slider
          label="Section gap"
          value={theme.sectionGapMm}
          min={0}
          max={12}
          step={0.5}
          suffix="mm"
          onChange={(sectionGapMm) => patchTheme({ sectionGapMm })}
        />
      </div>
    </section>
  )
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  suffix,
  onChange,
}: {
  label: string
  value: number
  min: number
  max: number
  step: number
  suffix: string
  onChange: (value: number) => void
}) {
  return (
    <div className="half">
      <label className="field">
        <span className="field-label">
          {label} <span className="muted">{value}{suffix}</span>
        </span>
        <input
          type="range"
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
        />
      </label>
    </div>
  )
}
