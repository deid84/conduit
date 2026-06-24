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
}

function makeLabel(conn: ConnConfig): string {
  switch (conn.type) {
    case 'serial': return `${conn.config.port} · ${conn.config.baud_rate}`
    case 'tcp':    return `${conn.config.host}:${conn.config.port}`
    case 'udp':    return `UDP ${conn.config.remote}`
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

  add(conn: ConnConfig): string {
    const id = crypto.randomUUID()
    this.connections.push({
      id,
      label:    makeLabel(conn),
      status:   'idle',
      conn,
      log:      [],
      rxBytes:  0,
      txBytes:  0,
    })
    this.activeId    = id
    this.newConnOpen = false
    return id
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

  appendLog(id: string, entry: LogEntry) {
    const conn = this.connections.find(c => c.id === id)
    if (!conn) return
    conn.log.push(entry)
    if (entry.direction === 'rx') conn.rxBytes += entry.raw.length
    else                          conn.txBytes += entry.raw.length
  },
})
