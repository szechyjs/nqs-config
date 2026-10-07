import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { NqsSnapshot } from '../types'

export const listPorts = (): Promise<string[]> =>
  invoke<string[]>('list_ports')

export const readConfig = (port: string): Promise<NqsSnapshot> =>
  invoke<NqsSnapshot>('read_config', { port })

export const writeConfig = (
  port: string,
  changes: Record<string, string>,
  force = false,
): Promise<NqsSnapshot> =>
  invoke<NqsSnapshot>('write_config', { port, changes, force })

export const onPortsChanged = (
  callback: (ports: string[]) => void,
): Promise<() => void> =>
  listen<string[]>('ports-changed', (e) => callback(e.payload))
