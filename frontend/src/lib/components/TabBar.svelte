<script lang="ts">
  import { store, disconnect } from '$lib/stores/connections.svelte'
  import { cn } from '$lib/utils'

  const STATUS_COLOR: Record<string, string> = {
    connected:    'bg-green-400',
    connecting:   'bg-yellow-400 animate-pulse',
    error:        'bg-red-400',
    disconnected: 'bg-zinc-600',
    idle:         'bg-zinc-600',
  }
</script>

<div class="flex h-9 items-stretch border-b border-border bg-muted/20 overflow-x-auto shrink-0">
  {#each store.connections as conn (conn.id)}
    <button
      class={cn(
        'group flex items-center gap-2 border-r border-border px-4 text-xs font-medium transition-colors whitespace-nowrap',
        conn.id === store.activeId
          ? 'bg-background text-foreground'
          : 'text-muted-foreground hover:text-foreground hover:bg-muted/50',
      )}
      onclick={() => { store.activeId = conn.id; store.newConnOpen = false }}
    >
      <span class={cn('size-1.5 shrink-0 rounded-full', STATUS_COLOR[conn.status])}></span>
      {conn.label}
      <span
        role="button"
        tabindex="0"
        class="ml-0.5 flex size-4 items-center justify-center rounded text-muted-foreground opacity-0 transition-opacity hover:bg-muted hover:text-foreground group-hover:opacity-100"
        onclick={(e) => { e.stopPropagation(); disconnect(conn.id) }}
        onkeydown={(e) => e.key === 'Enter' && (e.stopPropagation(), disconnect(conn.id))}
      >×</span>
    </button>
  {/each}

  <button
    class="flex items-center px-3 text-lg leading-none text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
    title="New connection"
    onclick={() => store.openNew()}
  >+</button>
</div>
