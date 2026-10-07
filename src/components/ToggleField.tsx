import type { FieldDef } from '../types'
import './ToggleField.css'

interface Props {
  def: FieldDef
  value: string           // current display value from snapshot
  pendingValue?: string   // if set, user has changed from read value
  onChange: (fieldName: string, writeValue: string) => void
  disabled: boolean
}

export function ToggleField({ def, value, pendingValue, onChange, disabled }: Props) {
  const activeValue = pendingValue ?? value
  const isChanged = pendingValue !== undefined && pendingValue !== value

  // Find which option index is active based on display value or pending write value
  function getActiveIndex(): number {
    if (pendingValue !== undefined) {
      return def.writeValues[1] === pendingValue ? 1 : 0
    }
    // Match display value to option
    return def.options[1] === value || value.toLowerCase().includes(def.options[1].toLowerCase()) ? 1 : 0
  }

  const activeIdx = getActiveIndex()
  const isDanger = def.danger !== undefined && activeValue === def.danger

  function handleClick(idx: number) {
    if (disabled) return
    onChange(def.fieldName, def.writeValues[idx])
  }

  return (
    <div className={`field-row ${isChanged ? 'field-row--changed' : ''}`}>
      <span className="field-label">{def.label}</span>
      <div className="toggle-group">
        <button
          className={`toggle-option ${activeIdx === 0 ? 'toggle-option--active' : ''}`}
          onClick={() => handleClick(0)}
          disabled={disabled}
          type="button"
        >
          {def.options[0]}
        </button>
        <button
          className={`toggle-option ${activeIdx === 1 ? 'toggle-option--active' : ''} ${activeIdx === 1 && isDanger ? 'toggle-option--danger' : ''}`}
          onClick={() => handleClick(1)}
          disabled={disabled}
          type="button"
        >
          {def.options[1]}
        </button>
      </div>
      {isDanger && (
        <span className="field-warning">⚠ This will mark the ECU as uninitialised</span>
      )}
    </div>
  )
}
