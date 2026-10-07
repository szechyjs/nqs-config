import { useState, useEffect, useCallback } from 'react'
import { confirm } from '@tauri-apps/plugin-dialog'
import { listPorts, readConfig, writeConfig, onPortsChanged } from '../api'
import { BYTE0_FIELDS, BYTE1_FIELDS, ENGINE_VARIANT_FIELD, type NqsSnapshot, type FieldKey } from '../types'
import { ToggleField } from '../components/ToggleField'
import { FieldSection } from '../components/FieldSection'
import { PortSelector } from '../components/PortSelector'
import { StatusBar, type StatusKind } from '../components/StatusBar'
import { HelpPanel } from '../components/HelpPanel'
import './Main.css'

interface Status {
  text: string
  kind: StatusKind
  detail?: string
}

export function Main() {
  const [ports, setPorts] = useState<string[]>([])
  const [port, setPort] = useState('')
  const [config, setConfig] = useState<NqsSnapshot | null>(null)
  const [pending, setPending] = useState<Record<string, string>>({})
  const [status, setStatus] = useState<Status>({ text: 'Ready', kind: 'idle' })
  const [busy, setBusy] = useState(false)
  const [showHelpPanel, setShowHelpPanel] = useState(false)

  // Load ports and listen for changes
  useEffect(() => {
    listPorts().then((ps) => {
      setPorts(ps)
      if (ps.length === 1) setPort(ps[0])
    })

    let unlisten: (() => void) | undefined
    onPortsChanged((ps) => {
      setPorts(ps)
      setPort((prev) => {
        if (ps.length === 1) return ps[0]
        if (!ps.includes(prev)) return ''
        return prev
      })
    }).then((fn) => { unlisten = fn })

    return () => { unlisten?.() }
  }, [])

  // Reset config, pending, and update status accuracy when port changes
  useEffect(() => {
    setConfig(null)
    setPending({})
    if (!port) {
      setStatus({ text: 'No device connected', kind: 'idle' })
    } else {
      setStatus({ text: 'Ready', kind: 'idle' })
    }
  }, [port])

  const handleRead = useCallback(async () => {
    if (!port) return
    setBusy(true)
    setStatus({ text: 'Reading ECU…', kind: 'busy' })
    setPending({})
    try {
      const snapshot = await readConfig(port)
      setConfig(snapshot)
      setStatus({ text: 'Read OK', kind: 'ok' })
    } catch (e) {
      setStatus({ text: 'Read failed', kind: 'error', detail: String(e) })
    } finally {
      setBusy(false)
    }
  }, [port])

  const handleFieldChange = useCallback((fieldName: string, writeValue: string) => {
    setPending((prev) => ({ ...prev, [fieldName]: writeValue }))
  }, [])

  const handleWrite = useCallback(async () => {
    if (!port || !config || Object.keys(pending).length === 0) return

    // Build a readable summary of changes
    const allFields = [...BYTE0_FIELDS, ...BYTE1_FIELDS]
    const summary = Object.entries(pending)
      .map(([fieldName, writeValue]) => {
        if (fieldName === 'engine_variant') {
          const label = writeValue === '4'
            ? ENGINE_VARIANT_FIELD.options[1]
            : ENGINE_VARIANT_FIELD.options[0]
          return `  • Engine Variant: ${label}`
        }
        const def = allFields.find((f) => f.fieldName === fieldName)
        if (!def) return `${fieldName} → ${writeValue}`
        const optIdx = def.writeValues[1] === writeValue ? 1 : 0
        return `  • ${def.label}: ${def.options[optIdx]}`
      })
      .join('\n')

    const ok = await confirm(
      `Write ${Object.keys(pending).length} change(s) to ECU EEPROM?\n\n${summary}\n\nThis cannot be undone.`,
      { title: 'Confirm Write', kind: 'warning' },
    )
    if (!ok) return

    setBusy(true)
    setStatus({ text: 'Writing ECU…', kind: 'busy' })

    // First pass — force=false so consistency check runs on the Rust side.
    try {
      const snapshot = await writeConfig(port, pending, false)
      setConfig(snapshot)
      setPending({})
      setStatus({ text: 'Write OK — ECU verified', kind: 'ok' })
      return
    } catch (e) {
      const msg = String(e)

      // If the Rust side returned a consistency warning, offer a bypass.
      if (msg.startsWith('CONSISTENCY_WARNING:')) {
        const warning = msg.replace('CONSISTENCY_WARNING:', '').trim()
        setBusy(false)

        const force = await confirm(
          `${warning}\n\nThis is expected for manual-swapped cars.\nWrite anyway?`,
          { title: 'Inconsistent Configuration', kind: 'warning' },
        )
        if (!force) {
          setStatus({ text: 'Write cancelled', kind: 'idle' })
          return
        }

        // Second pass — force=true bypasses the consistency check.
        setBusy(true)
        setStatus({ text: 'Writing ECU…', kind: 'busy' })
        try {
          const snapshot = await writeConfig(port, pending, true)
          setConfig(snapshot)
          setPending({})
          setStatus({ text: 'Write OK — ECU verified', kind: 'ok' })
        } catch (e2) {
          setStatus({ text: 'Write failed', kind: 'error', detail: String(e2) })
        }
        return
      }

      setStatus({ text: 'Write failed', kind: 'error', detail: msg })
    } finally {
      setBusy(false)
    }
  }, [port, config, pending])

  const pendingCount = Object.keys(pending).length
  const canRead = !!port && !busy
  const canWrite = !!port && !!config && pendingCount > 0 && !busy

  const getWriteTooltip = () => {
    if (!port) return 'No device connected'
    if (busy) return 'Operation in progress'
    if (!config) return 'Please read device configuration first'
    if (pendingCount === 0) return 'No changes to write'
    return undefined
  }

  // Helper to get the pending write value for a field key
  function getPending(key: FieldKey): string | undefined {
    const allFields = [...BYTE0_FIELDS, ...BYTE1_FIELDS]
    const def = allFields.find((f) => f.key === key)
    if (!def) return undefined
    return pending[def.fieldName]
  }

  return (
    <div className="app">
      <header className="app-header">
        <span className="app-title">NQS Config</span>
        <div className="app-header__right">
          <PortSelector
            ports={ports}
            value={port}
            onChange={setPort}
            disabled={busy}
          />
          <button
            className="btn-header"
            onClick={() => setShowHelpPanel((prev) => !prev)}
            type="button"
            title="Wiring help"
          >
            ❓ Help
          </button>
        </div>
      </header>

      <div className={`info-panel ${!showHelpPanel ? 'info-panel--hidden' : ''}`}>
        <HelpPanel />
      </div>

      <main className="app-main">
        {/* $22 Engine Variant */}
        <FieldSection
          title="$22 Engine Variant"
          rawLabel="raw"
          rawValue={config ? `0x${config.engine_variant_raw.toString(16).padStart(2, '0').toUpperCase()}` : '—'}
        >
          <ToggleField
            def={{ ...ENGINE_VARIANT_FIELD, key: 'engine_variant' as FieldKey }}
            value={config
              ? config.engine_variant_raw === 4
                ? ENGINE_VARIANT_FIELD.options[1]
                : ENGINE_VARIANT_FIELD.options[0]
              : ENGINE_VARIANT_FIELD.options[0]
            }
            pendingValue={pending['engine_variant']}
            onChange={handleFieldChange}
            disabled={!config || busy}
          />
        </FieldSection>

        {/* $24 Byte 0 */}
        <FieldSection
          title="$24 Byte 0"
          rawLabel="raw"
          rawValue={config ? `0x${config.byte0_raw}` : '—'}
        >
          {BYTE0_FIELDS.map((def) => (
            <ToggleField
              key={def.key}
              def={def}
              value={config ? config[def.key] : def.options[0]}
              pendingValue={getPending(def.key)}
              onChange={handleFieldChange}
              disabled={!config || busy}
            />
          ))}
        </FieldSection>

        {/* $24 Byte 1 */}
        <FieldSection
          title="$24 Byte 1"
          rawLabel="raw"
          rawValue={config ? `0x${config.byte1_raw}` : '—'}
        >
          {BYTE1_FIELDS.map((def) => (
            <ToggleField
              key={def.key}
              def={def}
              value={config ? config[def.key] : def.options[0]}
              pendingValue={getPending(def.key)}
              onChange={handleFieldChange}
              disabled={!config || busy}
            />
          ))}
        </FieldSection>
      </main>

      <footer className="app-footer">
        <div className={`app-actions ${!port ? 'app-actions--hidden' : ''}`}>
          <button
            className="btn btn--primary"
            onClick={handleRead}
            disabled={!canRead}
            type="button"
          >
            Read
          </button>
          <div className="app-actions__right">
            {pendingCount > 0 && (
              <span className="pending-badge">{pendingCount} change{pendingCount !== 1 ? 's' : ''}</span>
            )}
            <button
              className="btn btn--accent"
              onClick={handleWrite}
              disabled={!canWrite}
              title={getWriteTooltip()}
              type="button"
            >
              Write
            </button>
          </div>
        </div>
        <StatusBar text={status.text} kind={status.kind} detail={status.detail} />
      </footer>
    </div>
  )
}
