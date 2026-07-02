<script lang="ts">
  import type { Connection, ModbusFrame, ModbusPdu, ModbusMode } from '$lib/stores/connections.svelte'
  import { store } from '$lib/stores/connections.svelte'
  import { sendModbus } from '$lib/api'
  import { cn } from '$lib/utils'

  let { connection }: { connection: Connection } = $props()

  // ── Request builder state ──────────────────────────────────────────────────
  let unitId  = $state(1)
  let fc      = $state(3)         // default: Read Holding Registers
  let address = $state(0)
  let count   = $state(10)
  let value   = $state(0)
  let coilOn  = $state(false)
  let sending = $state(false)
  let txId    = $state(1)         // auto-increment for TCP

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
      case 15: return { type: 'write_multiple_coils',     address, count, values: Array(count).fill(coilOn) }
      case 16: return { type: 'write_multiple_registers', address, count, values: Array(count).fill(value) }
      default: return { type: 'unknown', function_code: fc, data: [] }
    }
  }

  async function sendRequest() {
    sending = true
    try {
      const pdu = buildPdu(fc)
      const tid = connection.modbusMode === 'tcp' ? txId++ : undefined
      await sendModbus(connection.id, unitId, pdu, tid)
      store.appendModbusTxFrame(connection.id, {
        mode:           connection.modbusMode!,
        unit_id:        unitId,
        transaction_id: tid ?? null,
        pdu,
      })
    } catch { /* ignore send errors */ } finally {
      sending = false
    }
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
        return pdu.values.map(v => `${v} (0x${v.toString(16).toUpperCase().padStart(4, '0')})`).join('  ')
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
        return pdu.values.map(v => v.toString(16).toUpperCase().padStart(4, '0')).join(' ')
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

      <!-- Count (read / write multiple) -->
      {#if needsCount || fc === 15 || fc === 16}
        <div class="flex flex-col gap-0.5">
          <label class="text-[10px] text-muted-foreground">Count</label>
          <input type="number" min="1" max="125" class="{INPUT_CLS} w-16" bind:value={count} />
        </div>
      {/if}

      <!-- Value / coil -->
      {#if isWrite && !needsCount}
        {#if isCoil}
          <label class="flex items-center gap-1.5 text-xs text-foreground cursor-pointer pb-1">
            <input type="checkbox" bind:checked={coilOn} />
            {coilOn ? 'ON' : 'OFF'}
          </label>
        {:else}
          <div class="flex flex-col gap-0.5">
            <label class="text-[10px] text-muted-foreground">Value</label>
            <input type="number" min="0" max="65535" class="{INPUT_CLS} w-20" bind:value />
          </div>
        {/if}
      {/if}

      <!-- TCP transaction ID -->
      {#if connection.modbusMode === 'tcp'}
        <div class="flex flex-col gap-0.5">
          <label class="text-[10px] text-muted-foreground">TxID</label>
          <input type="number" min="0" max="65535" class="{INPUT_CLS} w-16" bind:value={txId} />
        </div>
      {/if}

      <button
        class="rounded bg-primary px-3 py-1 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90 disabled:opacity-40"
        disabled={sending}
        onclick={sendRequest}
      >
        {sending ? '…' : 'Send'}
      </button>

      <button
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
          {#each connection.modbusFrames as frame (frame.ts + frame.direction + Math.random())}
            <tr class={cn(
              'border-b border-border/30',
              frame.direction === 'tx' ? 'text-cyan-400' : 'text-foreground'
            )}>
              <td class="px-3 py-0.5 text-muted-foreground">{fmtTime(frame.ts)}</td>
              <td class="px-2 py-0.5">
                <span class={frame.direction === 'tx' ? 'text-cyan-400' : 'text-green-400'}>
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
