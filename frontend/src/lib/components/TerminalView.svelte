<script lang="ts">
  import type { Connection } from '$lib/stores/connections.svelte'

  let { connection }: { connection: Connection } = $props()

  // Placeholder log to visualise the layout. Will be replaced by xterm.js.
  const SAMPLE = [
    { ts: '12:34:56.001', dir: 'rx', text: 'Conduit terminal ready' },
    { ts: '12:34:56.123', dir: 'rx', text: 'Waiting for data...' },
    { ts: '12:34:57.001', dir: 'tx', text: 'AT+VERSION\r' },
    { ts: '12:34:57.047', dir: 'rx', text: '+VERSION: 1.0.0' },
    { ts: '12:34:57.048', dir: 'rx', text: 'OK' },
    { ts: '12:34:58.200', dir: 'tx', text: 'AT+RESET\r' },
    { ts: '12:34:58.350', dir: 'rx', text: 'Rebooting...' },
  ]

  // Use real log entries if present, otherwise show the placeholder sample.
  const entries = $derived(
    connection.log.length > 0
      ? connection.log.map(e => ({
          ts:   new Date(e.ts).toISOString().slice(11, 23),
          dir:  e.direction,
          text: String.fromCharCode(...e.raw),
        }))
      : SAMPLE
  )

  let terminalEl = $state<HTMLDivElement | null>(null)
  $effect(() => {
    // Auto-scroll to bottom when entries change.
    void entries.length
    terminalEl?.scrollTo({ top: terminalEl.scrollHeight })
  })
</script>

<!--
  This div will be replaced by an xterm.js Terminal instance.
  The structure and styling below simulate how it will look.
-->
<div
  bind:this={terminalEl}
  class="flex-1 overflow-y-auto bg-zinc-950 px-3 py-2 font-mono text-xs leading-5"
>
  {#each entries as e}
    <div class="flex gap-3">
      <span class="shrink-0 select-none text-zinc-600">{e.ts}</span>
      <span class="shrink-0 select-none {e.dir === 'rx' ? 'text-zinc-500' : 'text-sky-500'}">
        {e.dir === 'rx' ? '←' : '→'}
      </span>
      <span class="{e.dir === 'rx' ? 'text-zinc-200' : 'text-sky-300'}">{e.text}</span>
    </div>
  {/each}
</div>
