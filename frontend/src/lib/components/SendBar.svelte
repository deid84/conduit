<script lang="ts">
  import { store } from '$lib/stores/connections.svelte'
  import { sendData } from '$lib/api'
  import { cn } from '$lib/utils'

  type Mode    = 'text' | 'hex'
  type LineEnd = 'none' | 'cr' | 'lf' | 'crlf'

  let mode:    Mode    = $state('text')
  let lineEnd: LineEnd = $state('lf')
  let input:   string  = $state('')

  const termMode = $derived(store.active?.terminalMode ?? 'line')

  // Valid when hex mode has at least one complete byte pair
  const hexValid = $derived(mode === 'hex' && /^([0-9a-fA-F]{2}\s*)+$/.test(input.trim()))
  const canSend  = $derived(store.active !== null && (mode === 'text' ? input.trim().length > 0 : hexValid))

  function toggleTermMode() {
    if (!store.activeId) return
    store.setTerminalMode(store.activeId, termMode === 'line' ? 'raw' : 'line')
  }

  // Normalize hex: strip non-hex, uppercase, insert space after every 2 chars.
  // Trailing space added automatically so cursor lands on the next pair.
  function onHexInput(e: Event) {
    const el  = e.target as HTMLInputElement
    const pos = el.selectionStart ?? el.value.length
    const beforeDigits = el.value.slice(0, pos).replace(/[^0-9a-fA-F]/gi, '')
    const raw    = el.value.replace(/[^0-9a-fA-F]/gi, '').toUpperCase()
    const pairs  = raw.match(/.{1,2}/g) ?? []
    const addTrailing = raw.length > 0 && raw.length % 2 === 0 && pos >= el.value.length
    input = pairs.join(' ') + (addTrailing ? ' ' : '')
    // Restore approximate cursor position
    const newPos = beforeDigits.length + Math.floor(beforeDigits.length / 2)
    requestAnimationFrame(() => el.setSelectionRange(newPos, newPos))
  }

  function onHexKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); return }
    // Pass through: navigation, backspace, delete, clipboard shortcuts
    if (e.ctrlKey || e.metaKey || e.altKey) return
    if (['Backspace', 'Delete', 'Tab', 'ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(e.key)) return
    if (!/^[0-9a-fA-F]$/.test(e.key)) e.preventDefault()
  }

  async function send() {
    if (!store.activeId || !canSend) return
    const id = store.activeId

    let bytes: Uint8Array
    if (mode === 'hex') {
      const digits = input.replace(/\s/g, '')
      bytes = new Uint8Array((digits.match(/.{2}/g) ?? []).map(h => parseInt(h, 16)))
    } else {
      const suffix = { none: '', cr: '\r', lf: '\n', crlf: '\r\n' }[lineEnd]
      bytes = new TextEncoder().encode(input + suffix)
    }

    input = ''
    store.appendLog(id, { ts: Date.now(), direction: 'tx', raw: Array.from(bytes) })
    await sendData(id, bytes).catch(err => console.error('send failed:', err))
  }

  const LINE_END_OPTS: { v: LineEnd; label: string }[] = [
    { v: 'none', label: 'No EOL' },
    { v: 'cr',   label: 'CR' },
    { v: 'lf',   label: 'LF' },
    { v: 'crlf', label: 'CR+LF' },
  ]
</script>

<div class="flex shrink-0 items-center gap-2 border-t border-border bg-background px-3 py-2">
  <!-- LINE / RAW terminal mode toggle -->
  <div class="flex rounded border border-border text-[10px]">
    {#each (['line', 'raw'] as const) as m}
      <button
        class={cn(
          'px-2 py-1 font-medium transition-colors',
          termMode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'
        )}
        title={m === 'raw' ? 'Raw mode: keystrokes go to device directly' : 'Line mode: compose and send lines'}
        onclick={toggleTermMode}
        disabled={!store.active}
      >{m.toUpperCase()}</button>
    {/each}
  </div>

  {#if termMode === 'line'}
    <!-- TEXT / HEX input mode toggle -->
    <div class="flex rounded border border-border text-[10px]">
      {#each (['text', 'hex'] as Mode[]) as m}
        <button
          class={cn(
            'px-2 py-1 font-medium transition-colors',
            mode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'
          )}
          onclick={() => { mode = m; input = '' }}
        >{m.toUpperCase()}</button>
      {/each}
    </div>

    {#if mode === 'text'}
      <select
        class="rounded border border-border bg-background px-1.5 py-1 text-[10px] text-muted-foreground focus:border-primary/60 focus:outline-none"
        bind:value={lineEnd}
      >
        {#each LINE_END_OPTS as opt}
          <option value={opt.v}>{opt.label}</option>
        {/each}
      </select>
    {/if}

    {#if mode === 'hex'}
      <input
        class="flex-1 rounded border border-border bg-muted/10 px-2 py-1.5 font-mono text-xs uppercase tracking-wider text-foreground placeholder:text-muted-foreground/40 focus:border-primary/60 focus:outline-none"
        placeholder="AA BB CC …"
        value={input}
        oninput={onHexInput}
        onkeydown={onHexKeydown}
        disabled={!store.active}
        spellcheck={false}
        autocomplete="off"
      />
    {:else}
      <input
        class="flex-1 rounded border border-border bg-muted/10 px-2 py-1.5 font-mono text-xs text-foreground placeholder:text-muted-foreground/40 focus:border-primary/60 focus:outline-none"
        placeholder="Type to send…"
        bind:value={input}
        onkeydown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send() } }}
        disabled={!store.active}
      />
    {/if}

    <button
      class="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90 active:opacity-75 disabled:cursor-not-allowed disabled:opacity-40"
      onclick={() => void send()}
      disabled={!canSend}
    >Send ⏎</button>

  {:else}
    <span class="flex-1 text-[11px] text-muted-foreground">
      Type directly in the terminal above — keystrokes go to the device in real time
    </span>
  {/if}
</div>
