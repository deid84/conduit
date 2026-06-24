<script lang="ts">
  import { store } from '$lib/stores/connections.svelte'
  import { cn } from '$lib/utils'

  type Mode    = 'text' | 'hex'
  type LineEnd = 'none' | 'cr' | 'lf' | 'crlf'

  let mode:    Mode    = $state('text')
  let lineEnd: LineEnd = $state('lf')
  let input:   string  = $state('')

  function send() {
    if (!input.trim()) return
    // TODO: POST to /api/connections/{id}/send
    store.appendLog(store.activeId!, {
      ts:        Date.now(),
      direction: 'tx',
      raw:       Array.from(new TextEncoder().encode(input + { none: '', cr: '\r', lf: '\n', crlf: '\r\n' }[lineEnd])),
    })
    input = ''
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send() }
  }

  const LINE_END_OPTS: { v: LineEnd; label: string }[] = [
    { v: 'none', label: 'No EOL' },
    { v: 'cr',   label: 'CR' },
    { v: 'lf',   label: 'LF' },
    { v: 'crlf', label: 'CR+LF' },
  ]
</script>

<div class="flex shrink-0 items-center gap-2 border-t border-border bg-background px-3 py-2">
  <!-- Mode toggle -->
  <div class="flex rounded border border-border text-[10px]">
    {#each (['text', 'hex'] as Mode[]) as m}
      <button
        class={cn(
          'px-2 py-1 font-medium transition-colors',
          mode === m ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'
        )}
        onclick={() => mode = m}
      >{m.toUpperCase()}</button>
    {/each}
  </div>

  <!-- Line ending -->
  <select
    class="rounded border border-border bg-background px-1.5 py-1 text-[10px] text-muted-foreground focus:border-primary/60 focus:outline-none"
    bind:value={lineEnd}
  >
    {#each LINE_END_OPTS as opt}
      <option value={opt.v}>{opt.label}</option>
    {/each}
  </select>

  <!-- Input -->
  <input
    class="flex-1 rounded border border-border bg-muted/10 px-2 py-1.5 font-mono text-xs text-foreground placeholder:text-muted-foreground/40 focus:border-primary/60 focus:outline-none"
    placeholder={mode === 'hex' ? 'AA BB CC …' : 'Type to send…'}
    bind:value={input}
    onkeydown={onKeydown}
    disabled={!store.active}
  />

  <!-- Send -->
  <button
    class="flex items-center gap-1.5 rounded bg-primary px-3 py-1.5 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90 active:opacity-75 disabled:cursor-not-allowed disabled:opacity-40"
    onclick={send}
    disabled={!store.active || !input.trim()}
  >Send ⏎</button>
</div>
