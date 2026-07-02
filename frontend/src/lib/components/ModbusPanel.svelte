<script lang="ts">
  import type { Connection, ModbusFrame, ModbusPdu, ModbusMode } from '$lib/stores/connections.svelte'
  import { store } from '$lib/stores/connections.svelte'
  import { sendModbus } from '$lib/api'
  import { cn } from '$lib/utils'

  let { connection }: { connection: Connection } = $props()

  // ── Connection settings ────────────────────────────────────────────────────
  let timeoutMs  = $state(1000)   // per-attempt timeout
  let retryCount = $state(3)      // max retries after first failure
  let sendError  = $state<string | null>(null)

  // ── Response display settings ──────────────────────────────────────────────
  type DisplayFormat = 'hex' | 'uint16' | 'int16' | 'uint32' | 'int32' | 'float32' | 'string'
  let displayFormat = $state<DisplayFormat>('hex')
  let swapBytes     = $state(false)  // swap bytes within each 16-bit word
  let swapWords     = $state(false)  // swap word order in 32-bit pairs (CDAB/DCBA)

  const is32bit = $derived(
    displayFormat === 'uint32' || displayFormat === 'int32' || displayFormat === 'float32'
  )

  // Convert an array of raw u16 register values to a display string using the
  // current format and byte/word-order settings.
  function wordsToDisplay(rawValues: number[]): string {
    // Step 1 — optional byte swap within each 16-bit word
    const words = swapBytes
      ? rawValues.map(v => ((v & 0xFF) << 8) | ((v >> 8) & 0xFF))
      : [...rawValues]

    switch (displayFormat) {
      case 'hex':
        return words.map(v => `0x${v.toString(16).toUpperCase().padStart(4, '0')}`).join('  ')

      case 'uint16':
        return words.map(v => String(v)).join('  ')

      case 'int16':
        return words.map(v => String(v >= 0x8000 ? v - 0x10000 : v)).join('  ')

      case 'string': {
        const bytes: number[] = []
        for (const v of words) bytes.push((v >> 8) & 0xFF, v & 0xFF)
        return bytes
          .filter(b => b !== 0)
          .map(b => b >= 0x20 && b < 0x7F ? String.fromCharCode(b) : '·')
          .join('')
      }

      default: {
        // Step 2 — optional word swap across pairs (for 32-bit formats)
        const w = swapWords ? [...words].reverse() : words
        const out: string[] = []
        for (let i = 0; i + 1 < w.length; i += 2) {
          const hi = w[i], lo = w[i + 1]
          const u32 = ((hi * 65536) + lo) >>> 0
          if (displayFormat === 'uint32') {
            out.push(String(u32))
          } else if (displayFormat === 'int32') {
            out.push(String(u32 >= 0x80000000 ? u32 - 0x100000000 : u32))
          } else {
            // float32 — pack bytes through DataView for correct IEEE 754 decoding
            const buf = new ArrayBuffer(4)
            const dv  = new DataView(buf)
            dv.setUint16(0, hi, false)
            dv.setUint16(2, lo, false)
            const f = dv.getFloat32(0, false)
            out.push(Number.isFinite(f) ? String(Number(f.toPrecision(7))) : String(f))
          }
        }
        // Leftover single word if odd register count — display as hex
        if (w.length % 2 !== 0) {
          const last = w[w.length - 1]
          out.push(`0x${last.toString(16).toUpperCase().padStart(4, '0')}`)
        }
        return out.length
          ? out.join('  ')
          : words.map(v => `0x${v.toString(16).toUpperCase().padStart(4, '0')}`).join('  ')
      }
    }
  }

  // ── Request builder state ──────────────────────────────────────────────────
  let unitId         = $state(1)
  let fc             = $state(3)   // default: Read Holding Registers
  let address        = $state(0)
  let count          = $state(10)
  let value          = $state(0)
  let coilOn         = $state(false)
  let multiValuesText = $state('')  // raw text for FC15/FC16 multi-value input
  let sending        = $state(false)
  let txId           = $state(1)   // auto-increment for TCP

  // ── Multi-value parsing ────────────────────────────────────────────────────

  // Tokenises a comma/whitespace-separated string, preserving quoted strings.
  function tokenizeValues(text: string): string[] {
    const tokens: string[] = []
    let i = 0
    while (i < text.length) {
      if (/[\s,]/.test(text[i])) { i++; continue }
      if (text[i] === '"' || text[i] === "'") {
        const q = text[i++]
        let tok = q
        while (i < text.length && text[i] !== q) tok += text[i++]
        tok += (text[i++] ?? '')
        tokens.push(tok)
      } else {
        let j = i
        while (j < text.length && !/[\s,]/.test(text[j])) j++
        tokens.push(text.slice(i, j))
        i = j
      }
    }
    return tokens
  }

  // Parse register values — supports decimal, 0x hex, and "ASCII" / 'ASCII'
  // strings (2 chars packed per u16, big-endian).
  function parseRegValues(text: string): number[] {
    const out: number[] = []
    for (const tok of tokenizeValues(text)) {
      if (/^0x[0-9a-fA-F]+$/i.test(tok)) {
        out.push(parseInt(tok, 16) & 0xFFFF)
      } else if (/^["'][^]*["']$/.test(tok)) {
        const str = tok.slice(1, -1)
        for (let i = 0; i < str.length; i += 2) {
          const hi = str.charCodeAt(i) & 0xFF
          const lo = i + 1 < str.length ? str.charCodeAt(i + 1) & 0xFF : 0
          out.push((hi << 8) | lo)
        }
      } else if (/^-?\d+$/.test(tok)) {
        const n = parseInt(tok, 10)
        out.push(((n % 65536) + 65536) % 65536)
      }
    }
    return out
  }

  // Parse coil values — 0/1, ON/OFF, true/false (case-insensitive).
  function parseCoilValues(text: string): boolean[] {
    return text.split(/[\s,]+/)
      .filter(Boolean)
      .map(t => t === '1' || /^(on|true)$/i.test(t))
  }

  const parsedRegs  = $derived(parseRegValues(multiValuesText))
  const parsedCoils = $derived(parseCoilValues(multiValuesText))
  const multiCount  = $derived(fc === 15 ? parsedCoils.length : parsedRegs.length)
  const canSend     = $derived(!sending && (fc !== 15 && fc !== 16 || multiCount > 0))

  const FC_OPTIONS = [
    { value: 1,  label: '01 – Read Coils' },
    { value: 2,  label: '02 – Read Discrete Inputs' },
    { value: 3,  label: '03 – Read Holding Registers' },
    { value: 4,  label: '04 – Read Input Registers' },
    { value: 5,  label: '05 – Write Single Coil' },
    { value: 6,  label: '06 – Write Single Register' },
    { value: 15, label: '0F – Write Multiple Coils' },
    { value: 16, label: '10 – Write Multiple Registers' },
  ]

  const needsCount = $derived([1, 2, 3, 4].includes(fc))
  const isCoil     = $derived([1, 2, 5, 15].includes(fc))
  const isWrite    = $derived([5, 6, 15, 16].includes(fc))

  function buildPdu(fc: number): ModbusPdu {
    switch (fc) {
      case 1:  return { type: 'read_bits',  fc: 1, start: address, count }
      case 2:  return { type: 'read_bits',  fc: 2, start: address, count }
      case 3:  return { type: 'read_words', fc: 3, start: address, count }
      case 4:  return { type: 'read_words', fc: 4, start: address, count }
      case 5:  return { type: 'write_single_coil',     address, on: coilOn }
      case 6:  return { type: 'write_single_register', address, value }
      case 15: return { type: 'write_multiple_coils',     address, count: parsedCoils.length, values: parsedCoils }
      case 16: return { type: 'write_multiple_registers', address, count: parsedRegs.length,  values: parsedRegs  }
      default: return { type: 'unknown', function_code: fc, data: [] }
    }
  }

  async function sendRequest() {
    // Snapshot reactive props once so the async path isn't affected by
    // re-renders or parent-driven prop changes mid-flight.
    const connId   = connection.id
    const connMode = connection.modbusMode
    if (!connId || !connMode) return

    sending   = true
    sendError = null
    const pdu = buildPdu(fc)

    let lastErr = ''
    const maxAttempts = 1 + Math.max(0, retryCount)

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      const ac = new AbortController()
      let timerId: ReturnType<typeof setTimeout> | undefined

      try {
        const tid = connMode === 'tcp' ? txId : undefined

        // Promise.race guarantees the await resolves within timeoutMs,
        // regardless of whether the AbortSignal is honoured by the runtime.
        // The no-op .catch() on the fetch suppresses unhandled-rejection warnings
        // in the (timeout-wins) case where the fetch eventually rejects after abort.
        const fetchPromise = sendModbus(connId, unitId, pdu, tid, ac.signal)
        fetchPromise.catch(() => {})
        await Promise.race([
          fetchPromise,
          new Promise<never>((_, reject) => {
            timerId = setTimeout(() => {
              ac.abort()
              reject(new Error(`timeout after ${timeoutMs}ms`))
            }, timeoutMs)
          }),
        ])

        clearTimeout(timerId)
        if (connMode === 'tcp') txId++
        store.appendModbusTxFrame(connId, {
          mode:           connMode,
          unit_id:        unitId,
          transaction_id: tid ?? null,
          pdu,
        })
        lastErr = ''
        break
      } catch (e) {
        clearTimeout(timerId)
        lastErr = e instanceof Error ? e.message : String(e)
        console.error(`[modbus] send attempt ${attempt + 1}/${maxAttempts}:`, e)
      }
    }

    if (lastErr) sendError = lastErr
    sending = false
  }

  // ── Frame display helpers ──────────────────────────────────────────────────
  function pduSummary(pdu: ModbusPdu): string {
    switch (pdu.type) {
      case 'read_bits':
      case 'read_words':
        return `@${pdu.start} × ${pdu.count}`
      case 'read_bits_response':
        return pdu.values.map(v => v ? '1' : '0').join(' ')
      case 'read_words_response':
        return wordsToDisplay(pdu.values)
      case 'write_single_coil':
        return `@${pdu.address} = ${pdu.on ? 'ON' : 'OFF'}`
      case 'write_single_register':
        return `@${pdu.address} = ${pdu.value} (0x${pdu.value.toString(16).toUpperCase().padStart(4, '0')})`
      case 'write_multiple_coils':
      case 'write_multiple_registers':
        return `@${pdu.address} × ${pdu.count}`
      case 'write_multiple_response':
        return `@${pdu.address} × ${pdu.count} ✓`
      case 'read_write_multiple':
        return `R@${pdu.read_address}×${pdu.read_count}  W@${pdu.write_address}`
      case 'read_write_multiple_response':
        return wordsToDisplay(pdu.values)
      case 'exception':
        return `EXC 0x${pdu.exception_code.toString(16).toUpperCase().padStart(2, '0')}`
      case 'unknown':
        return `FC 0x${pdu.function_code.toString(16).toUpperCase().padStart(2, '0')} [${pdu.data.length}B]`
    }
  }

  function fcName(pdu: ModbusPdu): string {
    switch (pdu.type) {
      case 'read_bits':
      case 'read_bits_response':
        return pdu.fc === 1 ? 'Read Coils' : 'Read DI'
      case 'read_words':
      case 'read_words_response':
        return pdu.fc === 3 ? 'Read HR' : 'Read IR'
      case 'write_single_coil':     return 'Write Coil'
      case 'write_single_register': return 'Write Reg'
      case 'write_multiple_coils':  return 'Write Coils'
      case 'write_multiple_registers': return 'Write Regs'
      case 'write_multiple_response':  return pdu.fc === 0x0F ? 'Write Coils ✓' : 'Write Regs ✓'
      case 'read_write_multiple':
      case 'read_write_multiple_response': return 'RW Multiple'
      case 'exception':  return 'Exception'
      case 'unknown':    return 'Unknown'
    }
  }

  function fmtTime(ts: number): string {
    const d = new Date(ts)
    return [d.getHours(), d.getMinutes(), d.getSeconds()]
      .map(n => String(n).padStart(2, '0')).join(':') +
      '.' + String(d.getMilliseconds()).padStart(3, '0')
  }

  const INPUT_CLS  = 'rounded border border-border bg-muted/20 px-2 py-1 text-xs text-foreground focus:border-primary/60 focus:outline-none'
  const SELECT_CLS = 'rounded border border-border bg-muted/20 px-2 py-1 text-xs text-foreground focus:border-primary/60 focus:outline-none'
</script>

<div class="flex flex-1 flex-col overflow-hidden">

  <!-- Settings bar: connection + display -->
  <div class="flex shrink-0 items-center gap-3 border-b border-border bg-muted/10 px-3 py-1.5 text-[11px]">
    <span class="text-muted-foreground">Timeout</span>
    <div class="flex items-center gap-1">
      <input
        type="number"
        min="100"
        max="60000"
        step="100"
        class="{INPUT_CLS} w-20 py-0.5 text-center"
        bind:value={timeoutMs}
      />
      <span class="text-muted-foreground">ms</span>
    </div>

    <span class="h-3 w-px bg-border"></span>

    <span class="text-muted-foreground">Retries</span>
    <input
      type="number"
      min="0"
      max="10"
      class="{INPUT_CLS} w-12 py-0.5 text-center"
      bind:value={retryCount}
    />

    <span class="h-3 w-px bg-border"></span>

    <span class="text-muted-foreground">Format</span>
    <select class="{SELECT_CLS} py-0" bind:value={displayFormat}>
      <option value="hex">Hex</option>
      <option value="uint16">Uint16</option>
      <option value="int16">Int16</option>
      <option value="uint32">Uint32</option>
      <option value="int32">Int32</option>
      <option value="float32">Float32</option>
      <option value="string">String</option>
    </select>

    <label class="flex cursor-pointer items-center gap-1.5">
      <input type="checkbox" class="h-3 w-3" bind:checked={swapBytes} />
      <span class="text-muted-foreground">Swap bytes</span>
    </label>

    {#if is32bit}
      <label class="flex cursor-pointer items-center gap-1.5">
        <input type="checkbox" class="h-3 w-3" bind:checked={swapWords} />
        <span class="text-muted-foreground">Swap words</span>
      </label>
    {/if}

    {#if sendError}
      <span class="ml-auto text-red-400">{sendError}</span>
    {/if}
  </div>

  <!-- Request builder -->
  <div class="shrink-0 border-b border-border bg-muted/5 px-3 py-2">
    <div class="flex flex-wrap items-end gap-2">

      <!-- Unit ID -->
      <div class="flex flex-col gap-0.5">
        <label class="text-[10px] text-muted-foreground">Unit ID</label>
        <input type="number" min="1" max="247" class="{INPUT_CLS} w-16" bind:value={unitId} />
      </div>

      <!-- Function code -->
      <div class="flex flex-col gap-0.5">
        <label class="text-[10px] text-muted-foreground">Function</label>
        <select class="{SELECT_CLS} w-52" bind:value={fc}>
          {#each FC_OPTIONS as opt}
            <option value={opt.value}>{opt.label}</option>
          {/each}
        </select>
      </div>

      <!-- Address -->
      <div class="flex flex-col gap-0.5">
        <label class="text-[10px] text-muted-foreground">{isWrite ? 'Address' : 'Start'}</label>
        <input type="number" min="0" max="65535" class="{INPUT_CLS} w-20" bind:value={address} />
      </div>

      <!-- Count — read FCs only (1-4) -->
      {#if needsCount}
        <div class="flex flex-col gap-0.5">
          <label class="text-[10px] text-muted-foreground">Count</label>
          <input type="number" min="1" max="125" class="{INPUT_CLS} w-16" bind:value={count} />
        </div>
      {/if}

      <!-- FC5: single coil -->
      {#if fc === 5}
        <label class="flex cursor-pointer items-center gap-1.5 pb-1 text-xs text-foreground">
          <input type="checkbox" bind:checked={coilOn} />
          {coilOn ? 'ON' : 'OFF'}
        </label>
      {/if}

      <!-- FC6: single register value -->
      {#if fc === 6}
        <div class="flex flex-col gap-0.5">
          <label class="text-[10px] text-muted-foreground">Value</label>
          <input type="number" min="0" max="65535" class="{INPUT_CLS} w-20" bind:value />
        </div>
      {/if}

      <!-- FC15/16: comma-separated values — decimal, 0x hex, "ASCII" -->
      {#if fc === 15 || fc === 16}
        <div class="flex flex-col gap-0.5">
          <label class="text-[10px] text-muted-foreground">
            Values <span class="opacity-50">({multiCount})</span>
          </label>
          <input
            type="text"
            placeholder={fc === 15 ? '1, 0, 1, 1' : '100, 0xFF, "AB"'}
            class="{INPUT_CLS} w-52"
            bind:value={multiValuesText}
          />
        </div>
      {/if}

      <!-- TCP transaction ID -->
      {#if connection.modbusMode === 'tcp'}
        <div class="flex flex-col gap-0.5">
          <label class="text-[10px] text-muted-foreground">TxID</label>
          <input type="number" min="0" max="65535" class="{INPUT_CLS} w-16" bind:value={txId} />
        </div>
      {/if}

      <button
        type="button"
        class="rounded bg-primary px-3 py-1 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
        disabled={!canSend}
        onclick={sendRequest}
      >
        {sending ? '…' : 'Send'}
      </button>

      <button
        type="button"
        class="ml-auto text-[10px] text-muted-foreground hover:text-foreground transition-colors"
        onclick={() => { const c = store.connections.find(c => c.id === connection.id); if (c) c.modbusFrames = [] }}
      >
        Clear
      </button>
    </div>
  </div>

  <!-- Frame log -->
  <div class="flex-1 overflow-y-auto font-mono text-[11px]">
    {#if connection.modbusFrames.length === 0}
      <div class="flex h-full items-center justify-center text-muted-foreground/50">
        No frames yet
      </div>
    {:else}
      <table class="w-full border-collapse">
        <thead class="sticky top-0 bg-background text-[10px] text-muted-foreground">
          <tr>
            <th class="px-3 py-1 text-left font-normal">Time</th>
            <th class="px-2 py-1 text-left font-normal">Dir</th>
            <th class="px-2 py-1 text-left font-normal">Unit</th>
            <th class="px-2 py-1 text-left font-normal">Function</th>
            <th class="px-3 py-1 text-left font-normal">Data</th>
          </tr>
        </thead>
        <tbody>
          {#each connection.modbusFrames as frame, i (i)}
            <tr class={cn(
              'border-b border-border/30',
              frame.direction === 'tx' ? 'text-tx' : 'text-foreground'
            )}>
              <td class="px-3 py-0.5 text-muted-foreground">{fmtTime(frame.ts)}</td>
              <td class="px-2 py-0.5">
                <span class={frame.direction === 'tx' ? 'text-tx' : 'text-rx'}>
                  {frame.direction === 'tx' ? '▲ TX' : '▼ RX'}
                </span>
              </td>
              <td class="px-2 py-0.5">{frame.unit_id}</td>
              <td class="px-2 py-0.5 text-muted-foreground">{fcName(frame.pdu)}</td>
              <td class="px-3 py-0.5 max-w-[40ch] truncate">{pduSummary(frame.pdu)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  </div>
</div>
