import { message } from '@tauri-apps/plugin-dialog'
import './StatusBar.css'

export type StatusKind = 'idle' | 'busy' | 'ok' | 'error'

interface Props {
  text: string
  kind: StatusKind
  detail?: string
}

export function StatusBar({ text, kind, detail }: Props) {
  const containerClass = `status-bar status-bar--${kind}`;

  const handleShowDetail = async () => {
    if (detail) {
      await message(detail, { title: 'Operation Failure Details', kind: 'error' })
    }
  }

  return (
    <div className={containerClass}>
      <div className="status-bar__left">
        {kind === 'busy' && <span className="status-bar__spinner" />}
        <span>{text}</span>
        {detail && (
          <button
            className="status-bar__detail-btn"
            onClick={handleShowDetail}
            type="button"
          >
            Details
          </button>
        )}
      </div>
    </div>
  )
}
