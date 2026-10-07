import './FieldSection.css'

interface Props {
  title: string
  rawLabel?: string
  rawValue?: string
  children: React.ReactNode
}

export function FieldSection({ title, rawLabel, rawValue, children }: Props) {
  return (
    <section className="field-section">
      <div className="field-section__header">
        <span className="field-section__title">{title}</span>
        {rawLabel && rawValue && (
          <span className="field-section__raw">
            <span className="field-section__raw-label">{rawLabel}</span>
            <code className="field-section__raw-value">{rawValue}</code>
          </span>
        )}
      </div>
      <div className="field-section__body">{children}</div>
    </section>
  )
}
