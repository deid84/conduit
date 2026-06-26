export type MacroMode    = 'text' | 'hex'
export type MacroLineEnd = 'none' | 'cr' | 'lf' | 'crlf'

export interface Macro {
  id: string
  label: string
  payload: string     // raw text, or hex string like "AA BB 0D"
  mode: MacroMode
  lineEnd: MacroLineEnd  // only used when mode === 'text'
}

const KEY = 'conduit_macros'

function load(): Macro[] {
  try { return JSON.parse(localStorage.getItem(KEY) ?? '[]') as Macro[] } catch { return [] }
}

export const macroStore = $state({ macros: load() })

function persist() {
  try { localStorage.setItem(KEY, JSON.stringify(macroStore.macros)) } catch { /* */ }
}

export function addMacro(m: Omit<Macro, 'id'>): void {
  macroStore.macros.push({ ...m, id: crypto.randomUUID() })
  persist()
}

export function removeMacro(id: string): void {
  const idx = macroStore.macros.findIndex(m => m.id === id)
  if (idx !== -1) { macroStore.macros.splice(idx, 1); persist() }
}

export function macroToBytes(macro: Macro): Uint8Array {
  if (macro.mode === 'hex') {
    const digits = macro.payload.replace(/\s/g, '')
    return new Uint8Array((digits.match(/.{2}/g) ?? []).map(h => parseInt(h, 16)))
  }
  const suffix = { none: '', cr: '\r', lf: '\n', crlf: '\r\n' }[macro.lineEnd]
  return new TextEncoder().encode(macro.payload + suffix)
}
