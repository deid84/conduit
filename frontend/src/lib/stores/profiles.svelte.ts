import type { ConnConfig } from './connections.svelte'

export interface Profile {
  id: string
  name: string
  config: ConnConfig
}

const STORAGE_KEY = 'conduit-profiles'

function load(): Profile[] {
  try {
    return JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '[]') as Profile[]
  } catch {
    return []
  }
}

function save(profiles: Profile[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(profiles))
}

export const profileStore = $state({
  profiles: load() as Profile[],

  add(name: string, config: ConnConfig): void {
    const profile: Profile = { id: crypto.randomUUID(), name, config }
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

  importJson(data: unknown): number {
    if (!Array.isArray(data)) throw new Error('expected array')
    const existingIds = new Set(this.profiles.map(p => p.id))
    let added = 0
    for (const p of data as Profile[]) {
      if (typeof p.id !== 'string' || typeof p.name !== 'string' || !p.config) continue
      if (existingIds.has(p.id)) continue
      this.profiles.push(p)
      existingIds.add(p.id)
      added++
    }
    save(this.profiles)
    return added
  },
})
