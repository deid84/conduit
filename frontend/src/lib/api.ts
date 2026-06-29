import type { ConnConfig, SerialConfig } from './stores/connections.svelte'

declare global {
  interface Window { __CONDUIT_API__?: string }
}

// Priority: Tauri injection → VITE_API_BASE env var → dev default.
export const BASE_URL =
  window.__CONDUIT_API__ ??
  (import.meta.env.VITE_API_BASE as string | undefined) ??
  'http://localhost:3000'

export const WS_BASE = BASE_URL.replace(/^http/, 'ws')

function serialApiBody(cfg: SerialConfig): Record<string, unknown> {
  const dataBits: Record<number, string> = { 5: 'five', 6: 'six', 7: 'seven', 8: 'eight' }
  const stopBits: Record<number, string> = { 1: 'one', 2: 'two' }
  return {
    port:         cfg.port,
    baud_rate:    cfg.baud_rate,
    data_bits:    dataBits[cfg.data_bits],
    stop_bits:    stopBits[cfg.stop_bits],
    parity:       cfg.parity,
    flow_control: cfg.flow_control,
  }
}

function toApiPayload(config: ConnConfig): unknown {
  if (config.type === 'serial') return { type: 'serial', config: serialApiBody(config.config) }
  return config
}

export async function openConnection(config: ConnConfig): Promise<{ id: string }> {
  const res = await fetch(`${BASE_URL}/api/connections`, {
    method:  'POST',
    headers: { 'Content-Type': 'application/json' },
    body:    JSON.stringify(toApiPayload(config)),
  })
  if (!res.ok) {
    const text = await res.text().catch(() => '')
    throw new Error(`open connection failed (${res.status})${text ? ': ' + text : ''}`)
  }
  return res.json()
}

export async function closeConnection(id: string): Promise<void> {
  await fetch(`${BASE_URL}/api/connections/${id}`, { method: 'DELETE' })
}

export async function sendData(id: string, data: Uint8Array): Promise<void> {
  await fetch(`${BASE_URL}/api/connections/${id}/send`, {
    method:  'POST',
    headers: { 'Content-Type': 'application/octet-stream' },
    body:    data as unknown as BodyInit,
  })
}

export async function listPorts(): Promise<Array<{ name: string }>> {
  const res = await fetch(`${BASE_URL}/api/ports`)
  if (!res.ok) throw new Error(`list ports failed (${res.status})`)
  return res.json()
}

export async function setDtr(id: string, value: boolean): Promise<void> {
  await fetch(`${BASE_URL}/api/connections/${id}/dtr`, {
    method:  'POST',
    headers: { 'Content-Type': 'application/json' },
    body:    JSON.stringify({ value }),
  })
}

export async function setRts(id: string, value: boolean): Promise<void> {
  await fetch(`${BASE_URL}/api/connections/${id}/rts`, {
    method:  'POST',
    headers: { 'Content-Type': 'application/json' },
    body:    JSON.stringify({ value }),
  })
}
