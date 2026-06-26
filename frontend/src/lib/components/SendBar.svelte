<script lang="ts">
  import { store, startFileLog, startFileLogFallback, stopAndSaveLog } from '$lib/stores/connections.svelte'
  import { sendData } from '$lib/api'
  import { cn } from '$lib/utils'

  const viewMode = $derived(store.active?.viewMode ?? 'ascii')

  type Mode    = 'text' | 'hex'
  type LineEnd = 'none' | 'cr' | 'lf' | 'crlf'

  let mode:    Mode    = $state('text')
  let lineEnd: LineEnd = $state('lf')
  let input:   string  = $state('')

  const termMode  = $derived(store.active?.terminalMode ?? 'line')
  const logActive = $derived(store.active?.fileLogging ?? false)

  // Fallback dialog (shown when File System Access API is unavailable)
  let recDialogOpen = $state(false)
  let recFilename   = $state('')

  function defaultFilename(): string {
    const conn = store.active
    if (!conn) return 'conduit.log'
    const safe = conn.label.replace(/[^a-zA-Z0-9._-]/g, '_')
    const date = new Date().toISOString().slice(0, 10)
    return `conduit-${safe}-${date}.log`
  }

  function clickRec() {
    if (logActive) {
      store.active && void stopAndSaveLog(store.active)
      return
    }
    if (!store.activeId) return
    if ('showSaveFilePicker' in window) {
      void startFileLog(store.activeId)
    } else {
      recFilename   = defaultFilename()
      recDialogOpen = true
    }
  }

  function confirmRec() {
    recDialogOpen = false
    if (!store.activeId || !recFilename.trim()) return
    startFileLogFallback(store.activeId, recFilename.trim())
  }

  // Valid when hex mode has at least one complete byte pair
  const hexValid = $derived(mode === 'hex' && /^([0-9a-fA-F]{2}\s*)+$/.test(input.trim()))
  const canSend  = $derived(store.active !== null && (mode === 'text' ? input.trim().length > 0 : hexValid))

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
  <!-- LINE / RAW terminal mode toggle — disabled entirely in HEX input mode -->
  <div
    class="flex rounded border border-border text-xs"
    title={mode === 'hex' ? 'Switch to TEXT input to change terminal mode' : undefined}
  >
    {#each (['line', 'raw'] as const) as m}
      <button
        class={cn(
          'px-2 py-1.5 font-medium transition-colors',
          termMode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'
        )}
        title={m === 'raw' ? 'Raw mode: keystrokes go to device directly' : 'Line mode: compose and send lines'}
        onclick={() => store.activeId && store.setTerminalMode(store.activeId, m)}
        disabled={!store.active || mode === 'hex'}
      >{m.toUpperCase()}</button>
    {/each}
  </div>

  {#if termMode === 'line'}
    <!-- TEXT / HEX input mode toggle -->
    <div class="flex rounded border border-border text-xs">
      {#each (['text', 'hex'] as Mode[]) as m}
        <button
          class={cn(
            'px-2 py-1.5 font-medium transition-colors',
            mode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'
          )}
          onclick={() => { mode = m; input = '' }}
        >{m.toUpperCase()}</button>
      {/each}
    </div>

    {#if mode === 'text'}
      <select
        class="rounded border border-border bg-background pl-1.5 pr-6 py-0.5 text-xs text-muted-foreground focus:border-primary/60 focus:outline-none"
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

  <!-- VIEW toggle: always visible, controls how received data is displayed -->
  <div class="ml-auto flex rounded border border-border text-xs" title="Terminal display mode">
    {#each (['ascii', 'hex'] as const) as m}
      <button
        class={cn(
          'px-2 py-1.5 font-mono font-medium transition-colors',
          viewMode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'
        )}
        onclick={() => store.activeId && store.setViewMode(store.activeId, m)}
        disabled={!store.active}
      >{m.toUpperCase()}</button>
    {/each}
  </div>

  <!-- REC: start/stop session log recording -->
  <button
    class={cn(
      'flex items-center gap-1.5 rounded border px-2 py-1.5 text-xs font-medium transition-colors disabled:opacity-40',
      logActive
        ? 'border-red-500/70 text-red-500'
        : 'border-border text-muted-foreground hover:text-foreground'
    )}
    title={logActive ? 'Stop recording and save file' : 'Start recording — choose where to save'}
    onclick={clickRec}
    disabled={!store.active}
  >
    <span class={cn('size-2 shrink-0 rounded-full', logActive ? 'bg-red-500 animate-pulse' : 'bg-muted-foreground')}></span>
    REC
  </button>

  <button
    class="rounded border border-destructive/50 px-2 py-1.5 text-xs text-destructive transition-colors hover:bg-destructive hover:text-white disabled:opacity-40"
    title="Clear terminal"
    onclick={() => store.activeId && store.clearLog(store.activeId)}
    disabled={!store.active}
  >Clear</button>
</div>

<!-- Fallback dialog: shown when showSaveFilePicker is not available (Firefox, WebKitGTK) -->
{#if recDialogOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
    onclick={() => recDialogOpen = false}
    role="dialog"
    aria-modal="true"
  >
    <div
      class="flex w-80 flex-col gap-3 rounded-lg border border-border bg-background p-4 shadow-xl"
      onclick={(e) => e.stopPropagation()}
    >
      <p class="text-sm font-semibold text-foreground">Start session recording</p>

      <div class="flex flex-col gap-1">
        <label class="text-xs text-muted-foreground" for="rec-filename">File name</label>
        <input
          id="rec-filename"
          class="rounded border border-border bg-input px-2 py-1.5 font-mono text-xs text-foreground focus:border-primary/60 focus:outline-none"
          bind:value={recFilename}
          onkeydown={(e) => { if (e.key === 'Enter') confirmRec(); if (e.key === 'Escape') recDialogOpen = false }}
          spellcheck={false}
          autocomplete="off"
        />
        <p class="text-[10px] text-muted-foreground/60">Saved to the browser's Downloads folder on stop</p>
      </div>

      <div class="flex justify-end gap-2">
        <button
          class="rounded border border-border px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground"
          onclick={() => recDialogOpen = false}
        >Cancel</button>
        <button
          class="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90"
          onclick={confirmRec}
        >
          <span class="size-1.5 rounded-full bg-red-500"></span>
          Start REC
        </button>
      </div>
    </div>
  </div>
{/if}
