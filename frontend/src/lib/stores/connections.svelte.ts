import { WS_BASE, openConnection, closeConnection } from '$lib/api'

export type ConnectionStatus = 'idle' | 'connecting' | 'connected' | 'error' | 'disconnected'

export interface SerialConfig {
  port: string
  baud_rate: number
  data_bits: 5 | 6 | 7 | 8
  stop_bits: 1 | 2
  parity: 'none' | 'odd' | 'even'
  flow_control: 'none' | 'software' | 'hardware'
}

export interface TcpConfig {
  host: string
  port: number
}

export interface UdpConfig {
  bind: string
  remote: string
}

export type ConnConfig =
  | { type: 'serial'; config: SerialConfig }
  | { type: 'tcp';    config: TcpConfig }
  | { type: 'udp';    config: UdpConfig }

export interface LogEntry {
  ts: number
  direction: 'rx' | 'tx'
  raw: number[]
}

export interface Connection {
  id: string
  label: string
  status: ConnectionStatus
  conn: ConnConfig
  log: LogEntry[]
  rxBytes: number
  txBytes: number
  terminalMode: 'line' | 'raw'
  viewMode: 'ascii' | 'hex'
}

function makeLabel(conn: ConnConfig): string {
  switch (conn.type) {
    case 'serial': return `${conn.config.port} · ${conn.config.baud_rate}`
    case 'tcp':    return `${conn.config.host}:${conn.config.port}`
    case 'udp':    return `UDP ${conn.config.remote}`
  }
}

// WebSocket registry — kept outside reactive state so sockets survive tab switches.
const _sockets = new Map<string, WebSocket>()

function openStream(id: string) {
  const ws = new WebSocket(`${WS_BASE}/api/connections/${id}/stream`)
  ws.binaryType = 'arraybuffer'
  ws.onmessage = (e) => {
    if (e.data instanceof ArrayBuffer) {
      store.appendLog(id, {
        ts:        Date.now(),
        direction: 'rx',
        raw:       Array.from(new Uint8Array(e.data)),
      })
    } else {
      try {
        const msg = JSON.parse(e.data as string) as { type?: string }
        if (msg.type === 'closed') store.setStatus(id, 'disconnected')
      } catch { /* ignore non-JSON text frames */ }
    }
  }
  ws.onclose = () => {
    store.setStatus(id, 'disconnected')
    _sockets.delete(id)
  }
  _sockets.set(id, ws)
}

export async function connect(config: ConnConfig): Promise<void> {
  const { id } = await openConnection(config)
  store.connections.push({
    id,
    label:        makeLabel(config),
    status:       'connected',
    conn:         config,
    log:          [],
    rxBytes:      0,
    txBytes:      0,
    terminalMode: 'line',
    viewMode:     'ascii',
  })
  store.activeId    = id
  store.newConnOpen = false
  openStream(id)
}

export async function disconnect(id: string): Promise<void> {
  _sockets.get(id)?.close()
  _sockets.delete(id)
  await closeConnection(id).catch(() => {})
  store.remove(id)
}

export const store = $state({
  connections: [] as Connection[],
  activeId:    null as string | null,
  newConnOpen: false,

  get active(): Connection | null {
    return this.connections.find(c => c.id === this.activeId) ?? null
  },

  openNew() {
    this.newConnOpen = true
    this.activeId = null
  },

  remove(id: string) {
    const idx = this.connections.findIndex(c => c.id === id)
    if (idx === -1) return
    this.connections.splice(idx, 1)
    if (this.activeId === id) {
      this.activeId = this.connections[Math.max(0, idx - 1)]?.id ?? null
    }
    if (this.connections.length === 0) this.newConnOpen = true
  },

  setStatus(id: string, status: ConnectionStatus) {
    const conn = this.connections.find(c => c.id === id)
    if (conn) conn.status = status
  },

  setTerminalMode(id: string, mode: 'line' | 'raw') {
    const conn = this.connections.find(c => c.id === id)
    if (conn) conn.terminalMode = mode
  },

  setViewMode(id: string, mode: 'ascii' | 'hex') {
    const conn = this.connections.find(c => c.id === id)
    if (conn) conn.viewMode = mode
  },

  appendLog(id: string, entry: LogEntry) {
    const conn = this.connections.find(c => c.id === id)
    if (!conn) return
    conn.log.push(entry)
    if (entry.direction === 'rx') conn.rxBytes += entry.raw.length
    else                          conn.txBytes += entry.raw.length
  },
})
