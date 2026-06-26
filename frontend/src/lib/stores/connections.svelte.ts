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
  fileLogging:  boolean
  logStart:     number        // log index when recording started; -1 = never recorded
  logFilename:  string        // chosen filename (used for fallback download)
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  fileHandle:   any | null    // FileSystemFileHandle when File System Access API is available
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
    fileLogging:  false,
    logStart:     -1,
    logFilename:  '',
    fileHandle:   null,
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

// ── File logging ────────────────────────────────────────────────────

function buildLogContent(conn: Connection, entries: LogEntry[]): string {
  const decoder = new TextDecoder('utf-8', { fatal: false })
  const header = [
    'Conduit session log',
    `Connection : ${conn.label}`,
    `Started    : ${new Date(entries[0].ts).toLocaleString()}`,
    '',
  ].join('\n')
  const lines = entries.map(e => {
    const d  = new Date(e.ts)
    const ts = [
      String(d.getHours()).padStart(2, '0'),
      String(d.getMinutes()).padStart(2, '0'),
      String(d.getSeconds()).padStart(2, '0'),
    ].join(':') + '.' + String(d.getMilliseconds()).padStart(3, '0')
    const dir  = e.direction === 'rx' ? 'RX' : 'TX'
    const text = decoder.decode(new Uint8Array(e.raw)).split('').map(ch => {
      const code = ch.charCodeAt(0)
      if (ch === '\n' || ch === '\r' || ch === '\t') return ch
      if (code < 0x20 || code === 0x7F) return `\\x${code.toString(16).padStart(2, '0')}`
      return ch
    }).join('')
    return `[${ts}] ${dir}  ${text}`
  })
  return header + lines.join('\n')
}

function fallbackDownload(content: string, filename: string): void {
  const blob = new Blob([content], { type: 'text/plain;charset=utf-8' })
  const url  = URL.createObjectURL(blob)
  const a    = document.createElement('a')
  a.href     = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}

// Shows the OS "Save as" dialog before starting; falls back to immediate
// recording (without a pre-chosen path) if the API is unavailable.
export async function startFileLog(id: string): Promise<void> {
  const conn = store.connections.find(c => c.id === id)
  if (!conn || conn.fileLogging) return

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  let handle: any = null
  if (typeof window !== 'undefined' && 'showSaveFilePicker' in window) {
    try {
      const safeName = conn.label.replace(/[^a-zA-Z0-9._-]/g, '_')
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      handle = await (window as any).showSaveFilePicker({
        suggestedName: `conduit-${safeName}-${new Date().toISOString().slice(0, 10)}.log`,
        types: [{ description: 'Log file', accept: { 'text/plain': ['.log', '.txt'] } }],
      })
    } catch {
      return  // user cancelled the dialog
    }
  }

  conn.fileHandle  = handle
  conn.logFilename = handle ? '' : ''   // filename set by caller for fallback path
  conn.fileLogging = true
  conn.logStart    = conn.log.length
}

// Used when File System Access API is unavailable: starts recording and
// stores the user-supplied filename for download on stop.
export function startFileLogFallback(id: string, filename: string): void {
  const conn = store.connections.find(c => c.id === id)
  if (!conn || conn.fileLogging) return
  conn.fileHandle  = null
  conn.logFilename = filename
  conn.fileLogging = true
  conn.logStart    = conn.log.length
}

// Stops recording and writes the log to the chosen file (or triggers a
// browser download when the File System Access API was not available).
export async function stopAndSaveLog(conn: Connection): Promise<void> {
  const handle   = conn.fileHandle
  const filename = conn.logFilename
  const entries  = conn.logStart >= 0 ? conn.log.slice(conn.logStart) : []

  conn.fileLogging = false
  conn.fileHandle  = null
  conn.logFilename = ''

  if (entries.length === 0) return
  const content = buildLogContent(conn, entries)

  if (handle) {
    try {
      const writable = await handle.createWritable()
      await writable.write(content)
      await writable.close()
    } catch {
      fallbackDownload(content, filename || `conduit-${conn.label.replace(/[^a-zA-Z0-9._-]/g, '_')}.log`)
    }
  } else {
    fallbackDownload(content, filename || `conduit-${conn.label.replace(/[^a-zA-Z0-9._-]/g, '_')}.log`)
  }
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

  clearLog(id: string) {
    const conn = this.connections.find(c => c.id === id)
    if (conn) conn.log = []
  },

  appendLog(id: string, entry: LogEntry) {
    const conn = this.connections.find(c => c.id === id)
    if (!conn) return
    conn.log.push(entry)
    if (entry.direction === 'rx') conn.rxBytes += entry.raw.length
    else                          conn.txBytes += entry.raw.length
  },
})
