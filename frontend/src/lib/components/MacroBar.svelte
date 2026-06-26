<script lang="ts">
  import { macroStore, addMacro, removeMacro, macroToBytes, type MacroMode, type MacroLineEnd } from '$lib/stores/macros.svelte'
  import { store as connStore } from '$lib/stores/connections.svelte'
  import { sendData } from '$lib/api'
  import { cn } from '$lib/utils'

  let adding    = $state(false)
  let newLabel   = $state('')
  let newPayload = $state('')
  let newMode    = $state<MacroMode>('text')
  let newLineEnd = $state<MacroLineEnd>('lf')

  const connected = $derived(connStore.active !== null)

  async function runMacro(id: string) {
    if (!connStore.activeId) return
    const macro = macroStore.macros.find(m => m.id === id)
    if (!macro) return
    const bytes = macroToBytes(macro)
    connStore.appendLog(connStore.activeId, { ts: Date.now(), direction: 'tx', raw: Array.from(bytes) })
    await sendData(connStore.activeId, bytes).catch(e => console.error('macro send failed:', e))
  }

  function confirmAdd() {
    const label   = newLabel.trim()
    const payload = newPayload.trim()
    if (!label || !payload) return
    addMacro({ label, payload, mode: newMode, lineEnd: newLineEnd })
    newLabel   = ''
    newPayload = ''
    newMode    = 'text'
    newLineEnd = 'lf'
    adding = false
  }

  function cancelAdd() {
    newLabel = ''; newPayload = ''; adding = false
  }

  async function sendFile() {
    if (!connStore.activeId) return
    const id  = connStore.activeId
    const inp = document.createElement('input')
    inp.type  = 'file'
    inp.onchange = async () => {
      const file = inp.files?.[0]
      if (!file) return
      const bytes = new Uint8Array(await file.arrayBuffer())
      // send in 4 kB chunks to avoid blocking the event loop
      const CHUNK = 4096
      for (let i = 0; i < bytes.length; i += CHUNK) {
        await sendData(id, bytes.slice(i, i + CHUNK))
      }
      connStore.appendLog(id, { ts: Date.now(), direction: 'tx', raw: Array.from(bytes) })
    }
    inp.click()
  }

  const LINE_ENDS: { v: MacroLineEnd; label: string }[] = [
    { v: 'none', label: 'No EOL' },
    { v: 'cr',   label: 'CR' },
    { v: 'lf',   label: 'LF' },
    { v: 'crlf', label: 'CR+LF' },
  ]
</script>

<div class="flex shrink-0 flex-col border-t border-border bg-muted/5">
  <!-- Add-macro form (shown above button row when adding) -->
  {#if adding}
    <div class="flex items-center gap-2 border-b border-border px-3 py-1.5">
      <input
        class="w-24 rounded border border-border bg-background px-2 py-1 text-xs text-foreground placeholder:text-muted-foreground/50 focus:border-primary/60 focus:outline-none"
        placeholder="Label"
        bind:value={newLabel}
        onkeydown={(e) => e.key === 'Enter' && confirmAdd()}
        autofocus
      />
      <input
        class="flex-1 rounded border border-border bg-background px-2 py-1 font-mono text-xs text-foreground placeholder:text-muted-foreground/50 focus:border-primary/60 focus:outline-none"
        placeholder={newMode === 'hex' ? 'AA BB CC …' : 'payload'}
        bind:value={newPayload}
        onkeydown={(e) => e.key === 'Enter' && confirmAdd()}
      />
      <!-- TEXT / HEX -->
      <div class="flex rounded border border-border text-[10px]">
        {#each (['text', 'hex'] as MacroMode[]) as m}
          <button
            class={cn('px-2 py-1 font-medium transition-colors',
              newMode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground')}
            onclick={() => newMode = m}
          >{m.toUpperCase()}</button>
        {/each}
      </div>
      {#if newMode === 'text'}
        <select
          class="rounded border border-border bg-background px-1.5 py-1 text-[10px] text-muted-foreground focus:outline-none"
          bind:value={newLineEnd}
        >
          {#each LINE_ENDS as opt}<option value={opt.v}>{opt.label}</option>{/each}
        </select>
      {/if}
      <button
        class="rounded bg-primary px-2 py-1 text-[10px] font-semibold text-primary-foreground disabled:opacity-40"
        onclick={confirmAdd}
        disabled={!newLabel.trim() || !newPayload.trim()}
      >Add</button>
      <button
        class="text-[10px] text-muted-foreground hover:text-foreground"
        onclick={cancelAdd}
      >Cancel</button>
    </div>
  {/if}

  <!-- Macro buttons row -->
  <div class="flex min-h-0 items-center gap-1.5 overflow-x-auto px-3 py-1.5">
    {#each macroStore.macros as macro (macro.id)}
      <div class="group relative shrink-0">
        <button
          class="rounded border border-border bg-background px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:bg-muted disabled:cursor-not-allowed disabled:opacity-40"
          title="{macro.mode === 'hex' ? '[HEX] ' : ''}{macro.payload}"
          onclick={() => runMacro(macro.id)}
          disabled={!connected}
        >{macro.label}</button>
        <!-- delete on hover -->
        <button
          class="absolute -right-1.5 -top-1.5 flex size-3.5 items-center justify-center rounded-full bg-destructive text-[9px] text-white opacity-0 transition-opacity group-hover:opacity-100"
          title="Remove macro"
          onclick={() => removeMacro(macro.id)}
        >×</button>
      </div>
    {/each}

    <button
      class="shrink-0 rounded border border-dashed border-border px-2 py-1 text-[10px] text-muted-foreground transition-colors hover:border-primary/60 hover:text-foreground"
      onclick={() => adding = true}
    >+ Macro</button>

    <!-- push send-file to the right -->
    <div class="flex-1"></div>

    <button
      class="shrink-0 flex items-center gap-1.5 rounded border border-border bg-background px-2.5 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:text-foreground disabled:cursor-not-allowed disabled:opacity-40"
      title="Send file contents to device"
      onclick={sendFile}
      disabled={!connected}
    >
      <svg class="size-3.5" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M2 12V14H14V12M8 2V10M5 7L8 10L11 7" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Send file
    </button>
  </div>
</div>
