import './PortSelector.css'

interface Props {
  ports: string[]
  value: string
  onChange: (port: string) => void
  disabled: boolean
}

export function PortSelector({ ports, value, onChange, disabled }: Props) {
  return (
    <div className="port-selector">
      <label className="port-selector__label">Port</label>
      <select
        className="port-selector__select"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
      >
        {ports.length === 0 && <option value="">No ports found</option>}
        {ports.length > 0 && !value && <option value="">Select port…</option>}
        {ports.map((p) => (
          <option key={p} value={p}>{p}</option>
        ))}
      </select>
    </div>
  )
}
