import type { ConnConfig, ModbusMode, MonitorRow } from './connections.svelte'

export interface Profile {
  id:          string
  name:        string
  config:      ConnConfig
  modbusMode:  ModbusMode | null
  monitorRows: MonitorRow[]
}

const STORAGE_KEY = 'conduit-profiles'

function normalize(p: Partial<Profile>): Profile | null {
  if (typeof p.id !== 'string' || typeof p.name !== 'string' || !p.config) return null
  return {
    id:          p.id,
    name:        p.name,
    config:      p.config,
    modbusMode:  p.modbusMode  ?? null,
    monitorRows: p.monitorRows ?? [],
  }
}

function load(): Profile[] {
  try {
    const raw = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]') as Partial<Profile>[]
    return raw.map(normalize).filter((p): p is Profile => p !== null)
  } catch {
    return []
  }
}

function save(profiles: Profile[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles))
}

export const profileStore = $state({
  profiles: load() as Profile[],

  add(name: string, config: ConnConfig, modbusMode: ModbusMode | null = null, monitorRows: MonitorRow[] = []): void {
    const profile: Profile = {
      id: crypto.randomUUID(),
      name,
      config,
      modbusMode,
      // strip runtime state before persisting
      monitorRows: monitorRows.map(({ values: _v, status: _s, lastTs: _t, errorMsg: _e, ...row }) => ({
        ...row, values: null, status: 'idle' as const, lastTs: null, errorMsg: null,
      })),
    }
    this.profiles.push(profile)
    save(this.profiles)
  },

  remove(id: string): void {
    const idx = this.profiles.findIndex(p => p.id === id)
    if (idx !== -1) {
      this.profiles.splice(idx, 1)
      save(this.profiles)
    }
  },

  update(id: string, changes: { config?: ConnConfig; modbusMode?: ModbusMode | null; monitorRows?: MonitorRow[] }): void {
    const idx = this.profiles.findIndex(p => p.id === id)
    if (idx === -1) return
    const existing = this.profiles[idx]
    this.profiles[idx] = {
      ...existing,
      ...(changes.config     !== undefined && { config:     changes.config }),
      ...(changes.modbusMode !== undefined && { modbusMode: changes.modbusMode }),
      monitorRows: changes.monitorRows !== undefined
        ? changes.monitorRows.map(({ values: _v, status: _s, lastTs: _t, errorMsg: _e, ...row }) => ({
            ...row, values: null, status: 'idle' as const, lastTs: null, errorMsg: null,
          }))
        : existing.monitorRows,
    }
    save(this.profiles)
  },

  importJson(data: unknown): number {
    if (!Array.isArray(data)) throw new Error('expected array')
    const existingIds = new Set(this.profiles.map(p => p.id))
    let added = 0
    for (const raw of data as Partial<Profile>[]) {
      const p = normalize(raw)
      if (!p || existingIds.has(p.id)) continue
      this.profiles.push(p)
      existingIds.add(p.id)
      added++
    }
    save(this.profiles)
    return added
  },
})
