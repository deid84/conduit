<script lang="ts">
  import type { Connection, ModbusPdu, MonitorRow, DisplayFormat } from '$lib/stores/connections.svelte'
  import { store } from '$lib/stores/connections.svelte'
  import { profileStore } from '$lib/stores/profiles.svelte'
  import { sendModbus, readModbus } from '$lib/api'
  import { cn } from '$lib/utils'

  let { connection }: { connection: Connection } = $props()

  // ── Shared ─────────────────────────────────────────────────────────────────
  let timeoutMs  = $state(1000)
  let retryCount = $state(3)
  let txId       = $state(1)   // TCP transaction counter, shared across both tabs
  let activeTab  = $state<'monitor' | 'log'>('monitor')

  const INPUT_CLS  = 'rounded border border-border bg-muted/20 px-2 py-1 text-xs text-foreground focus:border-primary/60 focus:outline-none'
  const SELECT_CLS = 'rounded border border-border bg-muted/20 px-2 py-1 text-xs text-foreground focus:border-primary/60 focus:outline-none'
  const CELL_INPUT  = 'w-full rounded border border-border bg-muted/20 px-1 py-0.5 text-[11px] text-foreground focus:border-primary/60 focus:outline-none disabled:opacity-40'
  const CELL_SELECT = 'w-full rounded border border-border bg-muted/20 px-1 py-0.5 text-[11px] text-foreground focus:border-primary/60 focus:outline-none disabled:opacity-40'

  // ── Display helper (parameterised — used by both tabs) ─────────────────────
  function wordsToDisplay(rawValues: number[], fmt: DisplayFormat, swapB: boolean, swapW: boolean): string {
    const words = swapB
      ? rawValues.map(v => ((v & 0xFF) << 8) | ((v >> 8) & 0xFF))
      : [...rawValues]

    switch (fmt) {
      case 'hex':
        return words.map(v => `0x${v.toString(16).toUpperCase().padStart(4, '0')}`).join('  ')
      case 'uint16':
        return words.map(v => String(v)).join('  ')
      case 'int16':
        return words.map(v => String(v >= 0x8000 ? v - 0x10000 : v)).join('  ')
      case 'string': {
        const bytes: number[] = []
        for (const v of words) bytes.push((v >> 8) & 0xFF, v & 0xFF)
        return bytes.filter(b => b !== 0).map(b => b >= 0x20 && b < 0x7F ? String.fromCharCode(b) : '·').join('')
      }
      default: {
        const w = swapW ? [...words].reverse() : words
        const out: string[] = []
        for (let i = 0; i + 1 < w.length; i += 2) {
          const hi = w[i], lo = w[i + 1]
          const u32 = ((hi * 65536) + lo) >>> 0
          if (fmt === 'uint32') {
            out.push(String(u32))
          } else if (fmt === 'int32') {
            out.push(String(u32 >= 0x80000000 ? u32 - 0x100000000 : u32))
          } else {
            const buf = new ArrayBuffer(4)
            const dv  = new DataView(buf)
            dv.setUint16(0, hi, false)
            dv.setUint16(2, lo, false)
            const f = dv.getFloat32(0, false)
            out.push(Number.isFinite(f) ? String(Number(f.toPrecision(7))) : String(f))
          }
        }
        if (w.length % 2 !== 0) out.push(`0x${w[w.length-1].toString(16).toUpperCase().padStart(4,'0')}`)
        return out.length
          ? out.join('  ')
          : words.map(v => `0x${v.toString(16).toUpperCase().padStart(4,'0')}`).join('  ')
      }
    }
  }

  function fmtTime(ts: number): string {
    const d = new Date(ts)
    return [d.getHours(), d.getMinutes(), d.getSeconds()]
      .map(n => String(n).padStart(2, '0')).join(':') +
      '.' + String(d.getMilliseconds()).padStart(3, '0')
  }

  // ── Save / update profile ──────────────────────────────────────────────────
  let savingProfileName = $state<string | null>(null)

  const linkedProfile = $derived(
    connection.sourceProfileId
      ? profileStore.profiles.find(p => p.id === connection.sourceProfileId) ?? null
      : null
  )

  function saveAsProfile() {
    const name = (savingProfileName ?? '').trim()
    if (!name) return
    profileStore.add(name, connection.conn, connection.modbusMode, connection.monitorRows)
    savingProfileName = null
  }

  function updateLinkedProfile() {
    if (!connection.sourceProfileId) return
    profileStore.update(connection.sourceProfileId, {
      config:      connection.conn,
      modbusMode:  connection.modbusMode,
      monitorRows: connection.monitorRows,
    })
  }

  // ── Monitor — polling ──────────────────────────────────────────────────────
  const POLL_INTERVALS = [
    { value: 100,   label: '100 ms' },
    { value: 200,   label: '200 ms' },
    { value: 500,   label: '500 ms' },
    { value: 1000,  label: '1 s' },
    { value: 2000,  label: '2 s' },
    { value: 5000,  label: '5 s' },
    { value: 10000, label: '10 s' },
  ]

  const MONITOR_FC_OPTIONS = [
    { value: 1, label: '01 Coils' },
    { value: 2, label: '02 DI' },
    { value: 3, label: '03 HR' },
    { value: 4, label: '04 IR' },
  ] as const

  const FORMAT_OPTIONS: { value: DisplayFormat; label: string }[] = [
    { value: 'hex',     label: 'Hex' },
    { value: 'uint16',  label: 'Uint16' },
    { value: 'int16',   label: 'Int16' },
    { value: 'uint32',  label: 'Uint32' },
    { value: 'int32',   label: 'Int32' },
    { value: 'float32', label: 'Float32' },
    { value: 'string',  label: 'String' },
  ]

  let pollTimer: ReturnType<typeof setTimeout> | null = null
  let cycleRunning = false
  let pollError = $state<string | null>(null)

  $effect(() => {
    if (connection.pollRunning) kickPoll()
    return () => {
      if (pollTimer !== null) { clearTimeout(pollTimer); pollTimer = null }
    }
  })

  function kickPoll() {
    if (!connection.pollRunning || cycleRunning) return
    runCycle().then(() => {
      if (connection.pollRunning) {
        pollTimer = setTimeout(kickPoll, connection.pollInterval)
      }
    })
  }

  async function runCycle() {
    cycleRunning = true
    pollError = null
    const connId   = connection.id
    const connMode = connection.modbusMode
    if (!connId || !connMode) { cycleRunning = false; return }

    try {
      for (const row of connection.monitorRows) {
        if (!connection.pollRunning) break
        row.status = 'pending'
        const pdu = buildRowPdu(row)
        const tid = connMode === 'tcp' ? txId++ : undefined
        store.appendModbusTxFrame(connId, {
          mode: connMode, unit_id: row.unitId, transaction_id: tid ?? null, pdu,
        })
        try {
          const result = await readModbus(connId, row.unitId, pdu, tid, timeoutMs)
          const rPdu = result.pdu as ModbusPdu
          row.lastTs = Date.now()
          if (rPdu.type === 'read_words_response') {
            row.values = rPdu.values; row.status = 'ok'; row.errorMsg = null
          } else if (rPdu.type === 'read_bits_response') {
            row.values = rPdu.values; row.status = 'ok'; row.errorMsg = null
          } else if (rPdu.type === 'exception') {
            row.values = null; row.status = 'error'
            row.errorMsg = `EXC 0x${rPdu.exception_code.toString(16).toUpperCase().padStart(2,'0')}`
          } else {
            row.values = null; row.status = 'error'; row.errorMsg = 'Unexpected response'
          }
        } catch (e) {
          const msg = e instanceof Error ? e.message : String(e)
          row.status   = msg.includes('408') ? 'timeout' : 'error'
          row.errorMsg = msg
          row.lastTs   = Date.now()
        }
      }
    } finally {
      cycleRunning = false
    }
  }

  function buildRowPdu(row: MonitorRow): ModbusPdu {
    if (row.fc === 1) return { type: 'read_bits',  fc: 1, start: row.address, count: row.count }
    if (row.fc === 2) return { type: 'read_bits',  fc: 2, start: row.address, count: row.count }
    if (row.fc === 3) return { type: 'read_words', fc: 3, start: row.address, count: row.count }
                      return { type: 'read_words', fc: 4, start: row.address, count: row.count }
  }

  function rowDisplayValue(row: MonitorRow): string {
    if (row.values === null) return '—'
    if (row.fc === 1 || row.fc === 2) {
      return (row.values as boolean[]).map(v => v ? '1' : '0').join(' ')
    }
    return wordsToDisplay(row.values as number[], row.displayFormat, row.swapBytes, row.swapWords)
  }

  function statusIcon(s: MonitorRow['status']): string {
    switch (s) {
      case 'idle':    return '○'
      case 'pending': return '⟳'
      case 'ok':      return '●'
      case 'error':   return '✕'
      case 'timeout': return '⏱'
    }
  }

  function statusCls(s: MonitorRow['status']): string {
    switch (s) {
      case 'idle':    return 'text-muted-foreground'
      case 'pending': return 'text-muted-foreground'
      case 'ok':      return 'text-rx'
      case 'error':   return 'text-destructive'
      case 'timeout': return 'text-primary'
    }
  }

  // ── Log tab — state ────────────────────────────────────────────────────────
  let displayFormat = $state<DisplayFormat>('hex')
  let swapBytes     = $state(false)
  let swapWords     = $state(false)
  let sendError     = $state<string | null>(null)

  let unitId          = $state(1)
  let fc              = $state(3)
  let address         = $state(0)
  let count           = $state(10)
  let value           = $state(0)
  let coilOn          = $state(false)
  let multiValuesText = $state('')
  let sending         = $state(false)

  const is32bit    = $derived(displayFormat === 'uint32' || displayFormat === 'int32' || displayFormat === 'float32')
  const needsCount = $derived([1, 2, 3, 4].includes(fc))
  const isWrite    = $derived([5, 6, 15, 16].includes(fc))
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

  function parseRegValues(text: string): number[] {
    const out: number[] = []
    for (const tok of tokenizeValues(text)) {
      if (/^0x[0-9a-fA-F]+$/i.test(tok)) out.push(parseInt(tok, 16) & 0xFFFF)
      else if (/^["'][^]*["']$/.test(tok)) {
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

  function parseCoilValues(text: string): boolean[] {
    return text.split(/[\s,]+/).filter(Boolean).map(t => t === '1' || /^(on|true)$/i.test(t))
  }

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
        const fetchPromise = sendModbus(connId, unitId, pdu, tid, ac.signal)
        fetchPromise.catch(() => {})
        await Promise.race([
          fetchPromise,
          new Promise<never>((_, reject) => {
            timerId = setTimeout(() => { ac.abort(); reject(new Error(`timeout after ${timeoutMs}ms`)) }, timeoutMs)
          }),
        ])
        clearTimeout(timerId)
        if (connMode === 'tcp') txId++
        store.appendModbusTxFrame(connId, { mode: connMode, unit_id: unitId, transaction_id: tid ?? null, pdu })
        lastErr = ''
        break
      } catch (e) {
        clearTimeout(timerId)
        lastErr = e instanceof Error ? e.message : String(e)
      }
    }

    if (lastErr) sendError = lastErr
    sending = false
  }

  // ── Frame log helpers ──────────────────────────────────────────────────────
  function pduSummary(pdu: ModbusPdu): string {
    switch (pdu.type) {
      case 'read_bits':
      case 'read_words':
        return `@${pdu.start} × ${pdu.count}`
      case 'read_bits_response':
        return pdu.values.map(v => v ? '1' : '0').join(' ')
      case 'read_words_response':
        return wordsToDisplay(pdu.values, displayFormat, swapBytes, swapWords)
      case 'write_single_coil':
        return `@${pdu.address} = ${pdu.on ? 'ON' : 'OFF'}`
      case 'write_single_register':
        return `@${pdu.address} = ${pdu.value} (0x${pdu.value.toString(16).toUpperCase().padStart(4,'0')})`
      case 'write_multiple_coils':
      case 'write_multiple_registers':
        return `@${pdu.address} × ${pdu.count}`
      case 'write_multiple_response':
        return `@${pdu.address} × ${pdu.count} ✓`
      case 'read_write_multiple':
        return `R@${pdu.read_address}×${pdu.read_count}  W@${pdu.write_address}`
      case 'read_write_multiple_response':
        return wordsToDisplay(pdu.values, displayFormat, swapBytes, swapWords)
      case 'exception':
        return `EXC 0x${pdu.exception_code.toString(16).toUpperCase().padStart(2,'0')}`
      case 'unknown':
        return `FC 0x${pdu.function_code.toString(16).toUpperCase().padStart(2,'0')} [${pdu.data.length}B]`
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
      case 'write_single_coil':         return 'Write Coil'
      case 'write_single_register':     return 'Write Reg'
      case 'write_multiple_coils':      return 'Write Coils'
      case 'write_multiple_registers':  return 'Write Regs'
      case 'write_multiple_response':   return pdu.fc === 0x0F ? 'Write Coils ✓' : 'Write Regs ✓'
      case 'read_write_multiple':
      case 'read_write_multiple_response': return 'RW Multiple'
      case 'exception': return 'Exception'
      case 'unknown':   return 'Unknown'
    }
  }
</script>

<div class="flex flex-1 flex-col overflow-hidden">

  <!-- Settings bar -->
  <div class="flex shrink-0 flex-wrap items-center gap-3 border-b border-border bg-muted/10 px-3 py-1.5 text-[11px]">
    <span class="text-muted-foreground">Timeout</span>
    <div class="flex items-center gap-1">
      <input type="number" min="100" max="60000" step="100"
        class="{INPUT_CLS} w-20 py-0.5 text-center" bind:value={timeoutMs} />
      <span class="text-muted-foreground">ms</span>
    </div>

    {#if activeTab === 'log'}
      <span class="h-3 w-px bg-border"></span>
      <span class="text-muted-foreground">Retries</span>
      <input type="number" min="0" max="10"
        class="{INPUT_CLS} w-12 py-0.5 text-center" bind:value={retryCount} />
      <span class="h-3 w-px bg-border"></span>
      <span class="text-muted-foreground">Format</span>
      <select class="{SELECT_CLS} py-0" bind:value={displayFormat}>
        {#each FORMAT_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
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
    {/if}

    {#if activeTab === 'log' && sendError}
      <span class="ml-auto text-destructive">{sendError}</span>
    {/if}
    {#if activeTab === 'monitor' && pollError}
      <span class="ml-auto text-destructive">{pollError}</span>
    {/if}
  </div>

  <!-- Tab bar -->
  <div class="flex shrink-0 border-b border-border">
    {#each (['monitor', 'log'] as const) as tab}
      <button
        type="button"
        class={cn(
          'border-b-2 px-4 py-1.5 text-[11px] font-medium transition-colors',
          activeTab === tab
            ? 'border-primary text-foreground'
            : 'border-transparent text-muted-foreground hover:text-foreground'
        )}
        onclick={() => { activeTab = tab }}
      >{tab === 'monitor' ? 'Monitor' : 'Log'}</button>
    {/each}
  </div>

  <!-- ── MONITOR TAB ──────────────────────────────────────────────────────── -->
  {#if activeTab === 'monitor'}

    <!-- Polling controls bar -->
    <div class="flex shrink-0 items-center gap-3 border-b border-border bg-muted/5 px-3 py-1.5 text-[11px]">
      <span class="text-muted-foreground">Interval</span>
      <select class="{SELECT_CLS} py-0" bind:value={connection.pollInterval}
        disabled={connection.pollRunning}>
        {#each POLL_INTERVALS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
      <button
        type="button"
        class={cn(
          'rounded px-3 py-0.5 text-xs font-semibold transition-colors',
          connection.pollRunning
            ? 'bg-destructive/80 text-white hover:bg-destructive'
            : 'bg-primary text-primary-foreground hover:opacity-90'
        )}
        onclick={() => { connection.pollRunning = !connection.pollRunning }}
        disabled={connection.monitorRows.length === 0}
      >{connection.pollRunning ? '■ Stop' : '▶ Start'}</button>
      {#if connection.pollRunning}
        <span class="text-muted-foreground">
          polling {connection.monitorRows.length} row{connection.monitorRows.length !== 1 ? 's' : ''}
        </span>
      {/if}
    </div>

    <!-- Monitor table -->
    <div class="flex-1 overflow-auto">
      {#if connection.monitorRows.length === 0}
        <div class="flex h-full items-center justify-center text-[11px] text-muted-foreground/50">
          No rows — click "+ Add row" to start monitoring
        </div>
      {:else}
        <table class="w-full border-collapse font-mono text-[11px]">
          <thead class="sticky top-0 bg-background text-[10px] text-muted-foreground">
            <tr>
              <th class="whitespace-nowrap px-2 py-1 text-left font-normal">Label</th>
              <th class="whitespace-nowrap px-2 py-1 text-left font-normal">FC</th>
              <th class="whitespace-nowrap px-2 py-1 text-left font-normal">Unit</th>
              <th class="whitespace-nowrap px-2 py-1 text-left font-normal">Addr</th>
              <th class="whitespace-nowrap px-2 py-1 text-left font-normal">Count</th>
              <th class="whitespace-nowrap px-2 py-1 text-left font-normal">Format</th>
              <th class="whitespace-nowrap px-2 py-1 text-center font-normal">↕B</th>
              <th class="whitespace-nowrap px-2 py-1 text-center font-normal">↕W</th>
              <th class="px-2 py-1 text-left font-normal">Value</th>
              <th class="whitespace-nowrap px-2 py-1 text-center font-normal">St</th>
              <th class="px-1 py-1"></th>
            </tr>
          </thead>
          <tbody>
            {#each connection.monitorRows as row (row.id)}
              <tr class="border-b border-border/30 hover:bg-muted/5">
                <td class="px-2 py-0.5">
                  <input type="text" placeholder="label…"
                    class="{CELL_INPUT} w-24"
                    bind:value={row.label}
                    disabled={connection.pollRunning} />
                </td>
                <td class="px-2 py-0.5">
                  <select class="{CELL_SELECT} w-[5.5rem]"
                    bind:value={row.fc}
                    disabled={connection.pollRunning}>
                    {#each MONITOR_FC_OPTIONS as opt}
                      <option value={opt.value}>{opt.label}</option>
                    {/each}
                  </select>
                </td>
                <td class="px-2 py-0.5">
                  <input type="number" min="1" max="247"
                    class="{CELL_INPUT} w-12 text-center"
                    bind:value={row.unitId}
                    disabled={connection.pollRunning} />
                </td>
                <td class="px-2 py-0.5">
                  <input type="number" min="0" max="65535"
                    class="{CELL_INPUT} w-16 text-center"
                    bind:value={row.address}
                    disabled={connection.pollRunning} />
                </td>
                <td class="px-2 py-0.5">
                  <input type="number" min="1" max="125"
                    class="{CELL_INPUT} w-12 text-center"
                    bind:value={row.count}
                    disabled={connection.pollRunning} />
                </td>
                <td class="px-2 py-0.5">
                  <select class="{CELL_SELECT} w-20"
                    bind:value={row.displayFormat}
                    disabled={connection.pollRunning}>
                    {#each FORMAT_OPTIONS as opt}
                      <option value={opt.value}>{opt.label}</option>
                    {/each}
                  </select>
                </td>
                <td class="px-2 py-0.5 text-center">
                  <input type="checkbox" class="h-3 w-3"
                    bind:checked={row.swapBytes}
                    disabled={connection.pollRunning} />
                </td>
                <td class="px-2 py-0.5 text-center">
                  <input type="checkbox" class="h-3 w-3"
                    bind:checked={row.swapWords}
                    disabled={connection.pollRunning || !['uint32','int32','float32'].includes(row.displayFormat)} />
                </td>
                <td class="max-w-0 truncate px-2 py-0.5"
                  title={row.status === 'error' || row.status === 'timeout' ? (row.errorMsg ?? undefined) : undefined}>
                  {#if row.status === 'error' || row.status === 'timeout'}
                    <span class="text-muted-foreground/70">{row.errorMsg ?? '—'}</span>
                  {:else}
                    {rowDisplayValue(row)}
                  {/if}
                </td>
                <td class="whitespace-nowrap px-2 py-0.5 text-center {statusCls(row.status)}"
                  title={row.lastTs ? fmtTime(row.lastTs) : undefined}>
                  {statusIcon(row.status)}
                </td>
                <td class="px-1 py-0.5">
                  <button type="button"
                    class="text-muted-foreground/50 transition-colors hover:text-destructive disabled:opacity-30"
                    onclick={() => store.deleteMonitorRow(connection.id, row.id)}
                    disabled={connection.pollRunning}>✕</button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

    <!-- Monitor footer: add row + save as profile -->
    <div class="shrink-0 border-t border-border px-3 py-1.5">
      {#if savingProfileName !== null}
        <div class="flex gap-1">
          <input
            type="text"
            placeholder="Profile name…"
            class="{CELL_INPUT} flex-1"
            bind:value={savingProfileName}
            onkeydown={(e) => { if (e.key === 'Enter') saveAsProfile(); if (e.key === 'Escape') savingProfileName = null }}
            autofocus
          />
          <button
            type="button"
            class="rounded border border-border px-2 py-0.5 text-[11px] text-foreground hover:bg-muted/40 transition-colors disabled:opacity-40"
            onclick={saveAsProfile}
            disabled={!savingProfileName?.trim()}
          >Save</button>
          <button
            type="button"
            class="rounded border border-border px-2 py-0.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
            onclick={() => savingProfileName = null}
          >✕</button>
        </div>
      {:else}
        <div class="flex items-center justify-between">
          <button
            type="button"
            class="text-[11px] text-muted-foreground transition-colors hover:text-foreground disabled:opacity-40"
            onclick={() => store.addMonitorRow(connection.id)}
            disabled={connection.pollRunning}
          >+ Add row</button>
          <div class="flex items-center gap-2">
            {#if linkedProfile}
              <button
                type="button"
                class="text-[11px] text-primary/80 transition-colors hover:text-primary"
                onclick={updateLinkedProfile}
                title="Overwrite profile '{linkedProfile.name}' with current settings and register map"
              >Update "{linkedProfile.name}"</button>
              <span class="text-muted-foreground/40">·</span>
            {/if}
            <button
              type="button"
              class="text-[11px] text-muted-foreground transition-colors hover:text-foreground"
              onclick={() => savingProfileName = ''}
              title="Save connection + register map as a new profile"
            >Save as profile…</button>
          </div>
        </div>
      {/if}
    </div>

  <!-- ── LOG TAB ──────────────────────────────────────────────────────────── -->
  {:else}

    <!-- Request builder -->
    <div class="shrink-0 border-b border-border bg-muted/5 px-3 py-2">
      <div class="flex flex-wrap items-end gap-2">

        <div class="flex flex-col gap-0.5">
          <span class="text-[10px] text-muted-foreground">Unit ID</span>
          <input type="number" min="1" max="247" class="{INPUT_CLS} w-16" bind:value={unitId} />
        </div>

        <div class="flex flex-col gap-0.5">
          <span class="text-[10px] text-muted-foreground">Function</span>
          <select class="{SELECT_CLS} w-52" bind:value={fc}>
            {#each FC_OPTIONS as opt}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        </div>

        <div class="flex flex-col gap-0.5">
          <span class="text-[10px] text-muted-foreground">{isWrite ? 'Address' : 'Start'}</span>
          <input type="number" min="0" max="65535" class="{INPUT_CLS} w-20" bind:value={address} />
        </div>

        {#if needsCount}
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] text-muted-foreground">Count</span>
            <input type="number" min="1" max="125" class="{INPUT_CLS} w-16" bind:value={count} />
          </div>
        {/if}

        {#if fc === 5}
          <label class="flex cursor-pointer items-center gap-1.5 pb-1 text-xs text-foreground">
            <input type="checkbox" bind:checked={coilOn} />
            {coilOn ? 'ON' : 'OFF'}
          </label>
        {/if}

        {#if fc === 6}
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] text-muted-foreground">Value</span>
            <input type="number" min="0" max="65535" class="{INPUT_CLS} w-20" bind:value />
          </div>
        {/if}

        {#if fc === 15 || fc === 16}
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] text-muted-foreground">
              Values <span class="opacity-50">({multiCount})</span>
            </span>
            <input type="text"
              placeholder={fc === 15 ? '1, 0, 1, 1' : '100, 0xFF, "AB"'}
              class="{INPUT_CLS} w-52"
              bind:value={multiValuesText} />
          </div>
        {/if}

        {#if connection.modbusMode === 'tcp'}
          <div class="flex flex-col gap-0.5">
            <span class="text-[10px] text-muted-foreground">TxID</span>
            <input type="number" min="0" max="65535" class="{INPUT_CLS} w-16" bind:value={txId} />
          </div>
        {/if}

        <button
          type="button"
          class="rounded bg-primary px-3 py-1 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
          disabled={!canSend} onclick={sendRequest}
        >{sending ? '…' : 'Send'}</button>

        <button
          type="button"
          class="ml-auto text-[10px] text-muted-foreground transition-colors hover:text-foreground"
          onclick={() => { const c = store.connections.find(c => c.id === connection.id); if (c) c.modbusFrames = [] }}
        >Clear</button>
      </div>
    </div>

    <!-- Frame log -->
    <div class="flex-1 overflow-y-auto font-mono text-[11px]">
      {#if connection.modbusFrames.length === 0}
        <div class="flex h-full items-center justify-center text-muted-foreground/50">No frames yet</div>
      {:else}
        <table class="w-full border-collapse">
          <thead class="sticky top-0 bg-background text-[10px] text-muted-foreground">
            <tr>
              <th class="w-px whitespace-nowrap px-4 py-1 text-left font-normal">Time</th>
              <th class="w-px whitespace-nowrap px-6 py-1 text-left font-normal">Dir</th>
              <th class="w-px whitespace-nowrap px-6 py-1 text-left font-normal">Unit</th>
              <th class="w-px whitespace-nowrap px-6 py-1 text-left font-normal">Function</th>
              <th class="px-6 py-1 text-left font-normal">Data</th>
            </tr>
          </thead>
          <tbody>
            {#each connection.modbusFrames as frame, i (i)}
              <tr class={cn('border-b border-border/30', frame.direction === 'tx' ? 'text-tx' : 'text-foreground')}>
                <td class="w-px whitespace-nowrap px-4 py-0.5 text-muted-foreground">{fmtTime(frame.ts)}</td>
                <td class="w-px whitespace-nowrap px-6 py-0.5">
                  <span class={frame.direction === 'tx' ? 'text-tx' : 'text-rx'}>
                    {frame.direction === 'tx' ? '▲ TX' : '▼ RX'}
                  </span>
                </td>
                <td class="w-px whitespace-nowrap px-6 py-0.5">{frame.unit_id}</td>
                <td class="w-px whitespace-nowrap px-6 py-0.5 text-muted-foreground">{fcName(frame.pdu)}</td>
                <td class="max-w-0 truncate px-6 py-0.5">{pduSummary(frame.pdu)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/if}
    </div>

  {/if}

</div>
